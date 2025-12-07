//! Converts Lua configuration values to Niri Config structures.
//!
//! This module provides utilities for extracting configuration values from a Lua runtime
//! and applying them to Niri's Config struct.

use log::{debug, info};
use niri_config::binds::{Action, Bind, Key, WorkspaceReference};
use niri_config::input::{
    AccelProfile, ClickMethod, FocusFollowsMouse, ScrollMethod, TapButtonMap, WarpMouseToFocus,
};
use niri_config::output::Position;
use niri_config::utils::Percent;
use niri_config::workspace::WorkspaceName;
use niri_config::{Config, FloatOrInt};
use niri_ipc::{ConfiguredMode, SizeChange, Transform};

use super::LuaRuntime;

/// Parse a size change string like "+10%", "-5%", "50%", "+100", "-50", "800"
fn parse_size_change(s: &str) -> Option<SizeChange> {
    let s = s.trim();

    if let Some(num_str) = s.strip_suffix('%') {
        // Percentage change
        if let Some(stripped) = num_str.strip_prefix('+') {
            let val: f64 = stripped.parse().ok()?;
            Some(SizeChange::AdjustProportion(val / 100.0))
        } else if let Some(stripped) = num_str.strip_prefix('-') {
            let val: f64 = stripped.parse().ok()?;
            Some(SizeChange::AdjustProportion(-val / 100.0))
        } else {
            let val: f64 = num_str.parse().ok()?;
            Some(SizeChange::SetProportion(val / 100.0))
        }
    } else {
        // Fixed pixel change
        if let Some(stripped) = s.strip_prefix('+') {
            let val: i32 = stripped.parse().ok()?;
            Some(SizeChange::AdjustFixed(val))
        } else if s.starts_with('-') {
            let val: i32 = s.parse().ok()?; // includes the minus sign
            Some(SizeChange::AdjustFixed(val))
        } else {
            let val: i32 = s.parse().ok()?;
            Some(SizeChange::SetFixed(val))
        }
    }
}

/// Apply pending configuration changes from the reactive config proxy.
///
/// This function applies changes that were captured via the `niri.config.*` reactive API
/// (e.g., `niri.config.layout.gaps = 16`, `niri.config.binds:add({...})`).
///
/// Scripts using the reactive API set values on the `niri.config.*` proxy tables,
/// which captures changes in `PendingConfigChanges`. This function then applies
/// those captured changes to the actual Config struct.
///
/// # Arguments
///
/// * `runtime` - The Lua runtime containing the pending config changes
/// * `config` - The config to apply changes to
///
/// # Returns
///
/// Returns the number of changes applied (scalar + collection additions).
pub fn apply_pending_lua_config(runtime: &LuaRuntime, config: &mut Config) -> usize {
    use crate::config_proxy::PendingConfigChanges;

    let pending: PendingConfigChanges = {
        let pending_ref = match &runtime.pending_config {
            Some(p) => p,
            None => {
                debug!("No pending config changes (proxy not initialized)");
                return 0;
            }
        };

        let mut pending = pending_ref.lock();
        if !pending.has_changes() {
            debug!("No pending config changes to apply");
            return 0;
        }

        // Take the changes and clear the pending state
        std::mem::take(&mut *pending)
    };

    let mut changes_applied = 0;

    info!(
        "Applying pending Lua config: {} scalar changes, {} collection additions, {} removals, {} replacements",
        pending.scalar_changes.len(),
        pending.collection_additions.values().map(|v| v.len()).sum::<usize>(),
        pending.collection_removals.values().map(|v| v.len()).sum::<usize>(),
        pending.collection_replacements.len()
    );

    // Apply scalar changes
    for (path, value) in &pending.scalar_changes {
        debug!("Applying scalar config change: {} = {:?}", path, value);

        let parts: Vec<&str> = path.split('.').collect();
        if parts.is_empty() {
            continue;
        }

        match parts[0] {
            "layout" => {
                if apply_layout_scalar_change(&mut config.layout, &parts[1..], value) {
                    changes_applied += 1;
                }
            }
            "animations" => {
                if apply_animation_scalar_change(&mut config.animations, &parts[1..], value) {
                    changes_applied += 1;
                }
            }
            "input" => {
                if apply_input_scalar_change(&mut config.input, &parts[1..], value) {
                    changes_applied += 1;
                }
            }
            "cursor" => {
                if apply_cursor_scalar_change(&mut config.cursor, &parts[1..], value) {
                    changes_applied += 1;
                }
            }
            "gestures" => {
                if apply_gestures_scalar_change(&mut config.gestures, &parts[1..], value) {
                    changes_applied += 1;
                }
            }
            "overview" => {
                if apply_overview_scalar_change(&mut config.overview, &parts[1..], value) {
                    changes_applied += 1;
                }
            }
            "recent_windows" => {
                if apply_recent_windows_scalar_change(
                    &mut config.recent_windows,
                    &parts[1..],
                    value,
                ) {
                    changes_applied += 1;
                }
            }
            "clipboard" => {
                if apply_clipboard_scalar_change(&mut config.clipboard, &parts[1..], value) {
                    changes_applied += 1;
                }
            }
            "hotkey_overlay" => {
                if apply_hotkey_overlay_scalar_change(
                    &mut config.hotkey_overlay,
                    &parts[1..],
                    value,
                ) {
                    changes_applied += 1;
                }
            }
            "config_notification" => {
                if apply_config_notification_scalar_change(
                    &mut config.config_notification,
                    &parts[1..],
                    value,
                ) {
                    changes_applied += 1;
                }
            }
            "debug" => {
                if apply_debug_scalar_change(&mut config.debug, &parts[1..], value) {
                    changes_applied += 1;
                }
            }
            "xwayland_satellite" => {
                if apply_xwayland_satellite_scalar_change(
                    &mut config.xwayland_satellite,
                    &parts[1..],
                    value,
                ) {
                    changes_applied += 1;
                }
            }
            "prefer_no_csd" => {
                if let Some(b) = value.as_bool() {
                    config.prefer_no_csd = b;
                    changes_applied += 1;
                }
            }
            "screenshot_path" => {
                if let Some(s) = value.as_str() {
                    config.screenshot_path = niri_config::ScreenshotPath(Some(s.to_string()));
                    changes_applied += 1;
                }
            }
            _ => {
                debug!("Unhandled config section: {}", parts[0]);
            }
        }
    }

    // Apply collection additions
    for (collection_name, items) in &pending.collection_additions {
        match collection_name.as_str() {
            "binds" => {
                for item in items {
                    if let Some(bind) = json_to_bind(item) {
                        config.binds.0.push(bind);
                        changes_applied += 1;
                        // debug!("Added bind from pending Lua config");
                    }
                }
            }
            "window_rules" => {
                for item in items {
                    if let Some(rule) = json_to_window_rule(item) {
                        config.window_rules.push(rule);
                        changes_applied += 1;
                        debug!("Added window rule from pending Lua config");
                    }
                }
            }
            "outputs" => {
                for item in items {
                    if let Some(output) = json_to_output(item) {
                        config.outputs.0.push(output);
                        changes_applied += 1;
                        debug!("Added output from pending Lua config");
                    }
                }
            }
            "spawn_at_startup" => {
                for item in items {
                    if let Some(spawn) = json_to_spawn(item) {
                        config.spawn_at_startup.push(spawn);
                        changes_applied += 1;
                        debug!("Added spawn_at_startup from pending Lua config");
                    }
                }
            }
            "environment" => {
                for item in items {
                    if let Some(env_var) = json_to_environment(item) {
                        config.environment.0.push(env_var);
                        changes_applied += 1;
                        debug!("Added environment variable from pending Lua config");
                    }
                }
            }
            "workspaces" => {
                for item in items {
                    if let Some(ws) = json_to_workspace(item) {
                        config.workspaces.push(ws);
                        changes_applied += 1;
                        debug!("Added workspace from pending Lua config");
                    }
                }
            }
            _ => {
                debug!("Unhandled collection addition: {}", collection_name);
            }
        }
    }

    // Apply collection removals
    for (collection_name, criteria_list) in &pending.collection_removals {
        match collection_name.as_str() {
            "binds" => {
                for criteria in criteria_list {
                    let before_len = config.binds.0.len();
                    config
                        .binds
                        .0
                        .retain(|bind| !bind_matches_criteria(bind, criteria));
                    let removed = before_len - config.binds.0.len();
                    if removed > 0 {
                        changes_applied += removed;
                        debug!("Removed {} bind(s) matching criteria", removed);
                    }
                }
            }
            "window_rules" => {
                for criteria in criteria_list {
                    let before_len = config.window_rules.len();
                    config
                        .window_rules
                        .retain(|rule| !window_rule_matches_criteria(rule, criteria));
                    let removed = before_len - config.window_rules.len();
                    if removed > 0 {
                        changes_applied += removed;
                        debug!("Removed {} window rule(s) matching criteria", removed);
                    }
                }
            }
            _ => {
                debug!("Unhandled collection removal: {}", collection_name);
            }
        }
    }

    // Apply collection replacements
    for (collection_name, items) in &pending.collection_replacements {
        match collection_name.as_str() {
            "binds" => {
                config.binds.0.clear();
                for item in items {
                    if let Some(bind) = json_to_bind(item) {
                        config.binds.0.push(bind);
                        changes_applied += 1;
                    }
                }
                debug!("Replaced all binds from pending Lua config");
            }
            "window_rules" => {
                config.window_rules.clear();
                for item in items {
                    if let Some(rule) = json_to_window_rule(item) {
                        config.window_rules.push(rule);
                        changes_applied += 1;
                    }
                }
                debug!("Replaced all window rules from pending Lua config");
            }
            _ => {
                debug!("Unhandled collection replacement: {}", collection_name);
            }
        }
    }

    info!("Applied {} pending Lua config changes", changes_applied);
    changes_applied
}

/// Apply a layout scalar change from JSON value.
fn apply_layout_scalar_change(
    layout: &mut niri_config::Layout,
    path: &[&str],
    value: &serde_json::Value,
) -> bool {
    if path.is_empty() {
        return false;
    }

    match path[0] {
        "gaps" => {
            if let Some(n) = value.as_f64() {
                layout.gaps = n;
                return true;
            } else if let Some(n) = value.as_i64() {
                layout.gaps = n as f64;
                return true;
            }
        }
        "struts" => {
            if path.len() > 1 {
                match path[1] {
                    "left" => {
                        if let Some(n) = value.as_f64() {
                            layout.struts.left = FloatOrInt(n);
                            return true;
                        } else if let Some(n) = value.as_i64() {
                            layout.struts.left = FloatOrInt(n as f64);
                            return true;
                        }
                    }
                    "right" => {
                        if let Some(n) = value.as_f64() {
                            layout.struts.right = FloatOrInt(n);
                            return true;
                        } else if let Some(n) = value.as_i64() {
                            layout.struts.right = FloatOrInt(n as f64);
                            return true;
                        }
                    }
                    "top" => {
                        if let Some(n) = value.as_f64() {
                            layout.struts.top = FloatOrInt(n);
                            return true;
                        } else if let Some(n) = value.as_i64() {
                            layout.struts.top = FloatOrInt(n as f64);
                            return true;
                        }
                    }
                    "bottom" => {
                        if let Some(n) = value.as_f64() {
                            layout.struts.bottom = FloatOrInt(n);
                            return true;
                        } else if let Some(n) = value.as_i64() {
                            layout.struts.bottom = FloatOrInt(n as f64);
                            return true;
                        }
                    }
                    _ => {}
                }
            }
        }
        "focus_ring" => {
            if path.len() > 1 {
                match path[1] {
                    "off" => {
                        if let Some(b) = value.as_bool() {
                            layout.focus_ring.off = b;
                            return true;
                        }
                    }
                    "width" => {
                        if let Some(n) = value.as_f64() {
                            layout.focus_ring.width = n;
                            return true;
                        } else if let Some(n) = value.as_i64() {
                            layout.focus_ring.width = n as f64;
                            return true;
                        }
                    }
                    "active_color" => {
                        if let Some(color) = json_to_color(value) {
                            layout.focus_ring.active_color = color;
                            return true;
                        }
                    }
                    "inactive_color" => {
                        if let Some(color) = json_to_color(value) {
                            layout.focus_ring.inactive_color = color;
                            return true;
                        }
                    }
                    _ => {}
                }
            }
        }
        "border" => {
            if path.len() > 1 {
                match path[1] {
                    "off" => {
                        if let Some(b) = value.as_bool() {
                            layout.border.off = b;
                            return true;
                        }
                    }
                    "width" => {
                        if let Some(n) = value.as_f64() {
                            layout.border.width = n;
                            return true;
                        } else if let Some(n) = value.as_i64() {
                            layout.border.width = n as f64;
                            return true;
                        }
                    }
                    "active_color" => {
                        if let Some(color) = json_to_color(value) {
                            layout.border.active_color = color;
                            return true;
                        }
                    }
                    "inactive_color" => {
                        if let Some(color) = json_to_color(value) {
                            layout.border.inactive_color = color;
                            return true;
                        }
                    }
                    "urgent_color" => {
                        if let Some(color) = json_to_color(value) {
                            layout.border.urgent_color = color;
                            return true;
                        }
                    }
                    _ => {}
                }
            }
        }
        "shadow" => {
            if path.len() > 1 {
                match path[1] {
                    "on" => {
                        if let Some(b) = value.as_bool() {
                            layout.shadow.on = b;
                            return true;
                        }
                    }
                    "softness" => {
                        if let Some(n) = value.as_f64() {
                            layout.shadow.softness = n;
                            return true;
                        } else if let Some(n) = value.as_i64() {
                            layout.shadow.softness = n as f64;
                            return true;
                        }
                    }
                    "spread" => {
                        if let Some(n) = value.as_f64() {
                            layout.shadow.spread = n;
                            return true;
                        } else if let Some(n) = value.as_i64() {
                            layout.shadow.spread = n as f64;
                            return true;
                        }
                    }
                    "offset" => {
                        if path.len() > 2 {
                            match path[2] {
                                "x" => {
                                    if let Some(n) = value.as_f64() {
                                        layout.shadow.offset.x = FloatOrInt(n);
                                        return true;
                                    } else if let Some(n) = value.as_i64() {
                                        layout.shadow.offset.x = FloatOrInt(n as f64);
                                        return true;
                                    }
                                }
                                "y" => {
                                    if let Some(n) = value.as_f64() {
                                        layout.shadow.offset.y = FloatOrInt(n);
                                        return true;
                                    } else if let Some(n) = value.as_i64() {
                                        layout.shadow.offset.y = FloatOrInt(n as f64);
                                        return true;
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                    "color" => {
                        if let Some(color) = json_to_color(value) {
                            layout.shadow.color = color;
                            return true;
                        }
                    }
                    "inactive_color" => {
                        if let Some(color) = json_to_color(value) {
                            layout.shadow.inactive_color = Some(color);
                            return true;
                        }
                    }
                    _ => {}
                }
            }
        }
        "always_center_single_column" => {
            if let Some(b) = value.as_bool() {
                layout.always_center_single_column = b;
                return true;
            }
        }
        "empty_workspace_above_first" => {
            if let Some(b) = value.as_bool() {
                layout.empty_workspace_above_first = b;
                return true;
            }
        }
        "center_focused_column" => {
            if let Some(s) = value.as_str() {
                match s {
                    "never" => {
                        layout.center_focused_column = niri_config::CenterFocusedColumn::Never;
                        return true;
                    }
                    "always" => {
                        layout.center_focused_column = niri_config::CenterFocusedColumn::Always;
                        return true;
                    }
                    "on-overflow" => {
                        layout.center_focused_column = niri_config::CenterFocusedColumn::OnOverflow;
                        return true;
                    }
                    _ => {}
                }
            }
        }
        "preset_column_widths" => {
            if let Some(arr) = value.as_array() {
                layout.preset_column_widths = arr.iter().filter_map(json_to_preset_size).collect();
                return true;
            }
        }
        "default_column_width" => {
            if let Some(size) = json_to_preset_size(value) {
                layout.default_column_width = Some(size);
                return true;
            }
        }
        "preset_window_heights" => {
            if let Some(arr) = value.as_array() {
                layout.preset_window_heights = arr.iter().filter_map(json_to_preset_size).collect();
                return true;
            }
        }
        _ => {}
    }
    false
}

/// Apply an animation scalar change from JSON value.
fn apply_animation_scalar_change(
    animations: &mut niri_config::Animations,
    path: &[&str],
    value: &serde_json::Value,
) -> bool {
    if path.is_empty() {
        return false;
    }

    match path[0] {
        "off" => {
            if let Some(b) = value.as_bool() {
                animations.off = b;
                return true;
            }
        }
        "slowdown" => {
            if let Some(n) = value.as_f64() {
                animations.slowdown = n;
                return true;
            } else if let Some(n) = value.as_i64() {
                animations.slowdown = n as f64;
                return true;
            }
        }
        _ => {}
    }
    false
}

/// Apply an input scalar change from JSON value.
fn apply_input_scalar_change(
    input: &mut niri_config::Input,
    path: &[&str],
    value: &serde_json::Value,
) -> bool {
    if path.is_empty() {
        return false;
    }

    match path[0] {
        "keyboard" => {
            if path.len() > 1 {
                match path[1] {
                    "repeat_delay" => {
                        if let Some(n) = value.as_u64() {
                            input.keyboard.repeat_delay = n as u16;
                            return true;
                        }
                    }
                    "repeat_rate" => {
                        if let Some(n) = value.as_u64() {
                            input.keyboard.repeat_rate = n as u8;
                            return true;
                        }
                    }
                    "numlock" => {
                        if let Some(b) = value.as_bool() {
                            input.keyboard.numlock = b;
                            return true;
                        }
                    }
                    "xkb" => {
                        if path.len() > 2 {
                            match path[2] {
                                "layout" => {
                                    if let Some(s) = value.as_str() {
                                        input.keyboard.xkb.layout = s.to_string();
                                        return true;
                                    }
                                }
                                "model" => {
                                    if let Some(s) = value.as_str() {
                                        input.keyboard.xkb.model = s.to_string();
                                        return true;
                                    }
                                }
                                "rules" => {
                                    if let Some(s) = value.as_str() {
                                        input.keyboard.xkb.rules = s.to_string();
                                        return true;
                                    }
                                }
                                "variant" => {
                                    if let Some(s) = value.as_str() {
                                        input.keyboard.xkb.variant = s.to_string();
                                        return true;
                                    }
                                }
                                "options" => {
                                    if let Some(s) = value.as_str() {
                                        input.keyboard.xkb.options = Some(s.to_string());
                                        return true;
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
        "touchpad" => {
            if path.len() > 1 {
                match path[1] {
                    "tap" => {
                        if let Some(b) = value.as_bool() {
                            input.touchpad.tap = b;
                            return true;
                        }
                    }
                    "natural_scroll" => {
                        if let Some(b) = value.as_bool() {
                            input.touchpad.natural_scroll = b;
                            return true;
                        }
                    }
                    "dwt" => {
                        if let Some(b) = value.as_bool() {
                            input.touchpad.dwt = b;
                            return true;
                        }
                    }
                    "dwtp" => {
                        if let Some(b) = value.as_bool() {
                            input.touchpad.dwtp = b;
                            return true;
                        }
                    }
                    "accel_speed" => {
                        if let Some(n) = value.as_f64() {
                            input.touchpad.accel_speed = FloatOrInt(n);
                            return true;
                        }
                    }
                    "accel_profile" => {
                        if let Some(s) = value.as_str() {
                            input.touchpad.accel_profile = match s {
                                "adaptive" => Some(AccelProfile::Adaptive),
                                "flat" => Some(AccelProfile::Flat),
                                _ => None,
                            };
                            return true;
                        }
                    }
                    "tap_button_map" => {
                        if let Some(s) = value.as_str() {
                            input.touchpad.tap_button_map = match s {
                                "left-right-middle" => Some(TapButtonMap::LeftRightMiddle),
                                "left-middle-right" => Some(TapButtonMap::LeftMiddleRight),
                                _ => None,
                            };
                            return true;
                        }
                    }
                    "scroll_method" => {
                        if let Some(s) = value.as_str() {
                            input.touchpad.scroll_method = match s {
                                "no-scroll" => Some(ScrollMethod::NoScroll),
                                "two-finger" => Some(ScrollMethod::TwoFinger),
                                "edge" => Some(ScrollMethod::Edge),
                                "on-button-down" => Some(ScrollMethod::OnButtonDown),
                                _ => None,
                            };
                            return true;
                        }
                    }
                    "disabled_on_external_mouse" => {
                        if let Some(b) = value.as_bool() {
                            input.touchpad.disabled_on_external_mouse = b;
                            return true;
                        }
                    }
                    "click_method" => {
                        if let Some(s) = value.as_str() {
                            input.touchpad.click_method = match s {
                                "button-areas" => Some(ClickMethod::ButtonAreas),
                                "clickfinger" => Some(ClickMethod::Clickfinger),
                                _ => None,
                            };
                            return true;
                        }
                    }
                    _ => {}
                }
            }
        }
        "mouse" => {
            if path.len() > 1 {
                match path[1] {
                    "natural_scroll" => {
                        if let Some(b) = value.as_bool() {
                            input.mouse.natural_scroll = b;
                            return true;
                        }
                    }
                    "accel_speed" => {
                        if let Some(n) = value.as_f64() {
                            input.mouse.accel_speed = FloatOrInt(n);
                            return true;
                        }
                    }
                    "accel_profile" => {
                        if let Some(s) = value.as_str() {
                            input.mouse.accel_profile = match s {
                                "adaptive" => Some(AccelProfile::Adaptive),
                                "flat" => Some(AccelProfile::Flat),
                                _ => None,
                            };
                            return true;
                        }
                    }
                    "scroll_method" => {
                        if let Some(s) = value.as_str() {
                            input.mouse.scroll_method = match s {
                                "no-scroll" => Some(ScrollMethod::NoScroll),
                                "two-finger" => Some(ScrollMethod::TwoFinger),
                                "edge" => Some(ScrollMethod::Edge),
                                "on-button-down" => Some(ScrollMethod::OnButtonDown),
                                _ => None,
                            };
                            return true;
                        }
                    }
                    _ => {}
                }
            }
        }
        "trackpoint" => {
            if path.len() > 1 {
                match path[1] {
                    "natural_scroll" => {
                        if let Some(b) = value.as_bool() {
                            input.trackpoint.natural_scroll = b;
                            return true;
                        }
                    }
                    "accel_speed" => {
                        if let Some(n) = value.as_f64() {
                            input.trackpoint.accel_speed = FloatOrInt(n);
                            return true;
                        }
                    }
                    "accel_profile" => {
                        if let Some(s) = value.as_str() {
                            input.trackpoint.accel_profile = match s {
                                "adaptive" => Some(AccelProfile::Adaptive),
                                "flat" => Some(AccelProfile::Flat),
                                _ => None,
                            };
                            return true;
                        }
                    }
                    "scroll_method" => {
                        if let Some(s) = value.as_str() {
                            input.trackpoint.scroll_method = match s {
                                "no-scroll" => Some(ScrollMethod::NoScroll),
                                "on-button-down" => Some(ScrollMethod::OnButtonDown),
                                _ => None,
                            };
                            return true;
                        }
                    }
                    _ => {}
                }
            }
        }
        "touch" => {
            if path.len() > 1 {
                match path[1] {
                    "off" => {
                        if let Some(b) = value.as_bool() {
                            input.touch.off = b;
                            return true;
                        }
                    }
                    "natural_scroll" => {
                        if let Some(b) = value.as_bool() {
                            input.touch.natural_scroll = b;
                            return true;
                        }
                    }
                    "map_to_output" => {
                        if let Some(s) = value.as_str() {
                            input.touch.map_to_output = Some(s.to_string());
                            return true;
                        }
                    }
                    _ => {}
                }
            }
        }
        "workspace_auto_back_and_forth" => {
            if let Some(b) = value.as_bool() {
                input.workspace_auto_back_and_forth = b;
                return true;
            }
        }
        "focus_follows_mouse" => {
            if let Some(obj) = value.as_object() {
                let max_scroll_amount = obj
                    .get("max_scroll_amount")
                    .and_then(|v| v.as_f64())
                    .map(Percent);
                input.focus_follows_mouse = Some(FocusFollowsMouse { max_scroll_amount });
                return true;
            } else if let Some(b) = value.as_bool() {
                if b {
                    input.focus_follows_mouse = Some(FocusFollowsMouse {
                        max_scroll_amount: None,
                    });
                } else {
                    input.focus_follows_mouse = None;
                }
                return true;
            }
        }
        "warp_mouse_to_focus" => {
            if let Some(b) = value.as_bool() {
                if b {
                    input.warp_mouse_to_focus = Some(WarpMouseToFocus { mode: None });
                } else {
                    input.warp_mouse_to_focus = None;
                }
                return true;
            }
        }
        _ => {}
    }
    false
}

/// Apply a cursor scalar change from JSON value.
fn apply_cursor_scalar_change(
    cursor: &mut niri_config::Cursor,
    path: &[&str],
    value: &serde_json::Value,
) -> bool {
    if path.is_empty() {
        return false;
    }

    match path[0] {
        "xcursor_theme" => {
            if let Some(s) = value.as_str() {
                cursor.xcursor_theme = s.to_string();
                return true;
            }
        }
        "xcursor_size" => {
            if let Some(n) = value.as_u64() {
                cursor.xcursor_size = n as u8;
                return true;
            }
        }
        "hide_when_typing" => {
            if let Some(b) = value.as_bool() {
                cursor.hide_when_typing = b;
                return true;
            }
        }
        "hide_after_inactive_ms" => {
            if let Some(n) = value.as_u64() {
                cursor.hide_after_inactive_ms = Some(n as u32);
                return true;
            }
        }
        _ => {}
    }
    false
}

/// Convert a JSON value to a Bind struct.
fn json_to_bind(json: &serde_json::Value) -> Option<Bind> {
    let obj = json.as_object()?;

    // Parse the key string (e.g., "Mod+T")
    let key_str = obj.get("key")?.as_str()?;
    let key: Key = key_str.parse().ok()?;

    // Parse the action
    let action_str = obj.get("action")?.as_str()?;
    let args: Vec<String> = obj
        .get("args")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| {
                    // Handle both string and numeric args
                    if let Some(s) = v.as_str() {
                        Some(s.to_string())
                    } else if let Some(n) = v.as_i64() {
                        Some(n.to_string())
                    } else if let Some(n) = v.as_u64() {
                        Some(n.to_string())
                    } else {
                        v.as_f64().map(|n| n.to_string())
                    }
                })
                .collect()
        })
        .unwrap_or_default();

    let action = parse_action_from_str(action_str, &args)?;

    // Parse optional fields
    let repeat = obj.get("repeat").and_then(|v| v.as_bool()).unwrap_or(true);
    let allow_when_locked = obj
        .get("allow_when_locked")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let allow_inhibiting = obj
        .get("allow_inhibiting")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);

    Some(Bind {
        key,
        action,
        repeat,
        cooldown: None,
        allow_when_locked,
        allow_inhibiting,
        hotkey_overlay_title: None,
    })
}

/// Parse an action string and arguments into an Action enum.
fn parse_action_from_str(action_str: &str, args: &[String]) -> Option<Action> {
    Some(match action_str {
        "spawn" => {
            if args.is_empty() {
                return None;
            }
            Action::Spawn(args.to_vec())
        }
        "spawn-sh" => {
            if args.is_empty() {
                return None;
            }
            Action::SpawnSh(args.join(" "))
        }
        "close-window" => Action::CloseWindow,
        "fullscreen-window" => Action::FullscreenWindow,
        "toggle-windowed-fullscreen" => Action::ToggleWindowedFullscreen,
        "toggle-window-floating" => Action::ToggleWindowFloating,
        "maximize-column" => Action::MaximizeColumn,
        "center-column" => Action::CenterColumn,
        "focus-window-down" => Action::FocusWindowDown,
        "focus-window-up" => Action::FocusWindowUp,
        "focus-column-left" => Action::FocusColumnLeft,
        "focus-column-right" => Action::FocusColumnRight,
        "focus-column-first" => Action::FocusColumnFirst,
        "focus-column-last" => Action::FocusColumnLast,
        "move-column-left" => Action::MoveColumnLeft,
        "move-column-right" => Action::MoveColumnRight,
        "move-column-to-first" => Action::MoveColumnToFirst,
        "move-column-to-last" => Action::MoveColumnToLast,
        "move-window-down" => Action::MoveWindowDown,
        "move-window-up" => Action::MoveWindowUp,
        "focus-workspace-down" => Action::FocusWorkspaceDown,
        "focus-workspace-up" => Action::FocusWorkspaceUp,
        "move-workspace-down" => Action::MoveWorkspaceDown,
        "move-workspace-up" => Action::MoveWorkspaceUp,
        "move-window-to-workspace-down" => Action::MoveWindowToWorkspaceDown(false),
        "move-window-to-workspace-up" => Action::MoveWindowToWorkspaceUp(false),
        "move-column-to-workspace-down" => Action::MoveColumnToWorkspaceDown(true),
        "move-column-to-workspace-up" => Action::MoveColumnToWorkspaceUp(true),
        "focus-monitor-left" => Action::FocusMonitorLeft,
        "focus-monitor-right" => Action::FocusMonitorRight,
        "focus-monitor-down" => Action::FocusMonitorDown,
        "focus-monitor-up" => Action::FocusMonitorUp,
        "move-column-to-monitor-left" => Action::MoveColumnToMonitorLeft,
        "move-column-to-monitor-right" => Action::MoveColumnToMonitorRight,
        "move-column-to-monitor-down" => Action::MoveColumnToMonitorDown,
        "move-column-to-monitor-up" => Action::MoveColumnToMonitorUp,
        "toggle-overview" => Action::ToggleOverview {},
        "open-overview" => Action::OpenOverview,
        "close-overview" => Action::CloseOverview,
        "quit" | "exit" => Action::Quit(false),
        "power-off-monitors" => Action::PowerOffMonitors,
        "power-on-monitors" => Action::PowerOnMonitors,
        "screenshot" => Action::Screenshot(true, None),
        "screenshot-screen" => Action::ScreenshotScreen(true, true, None),
        "screenshot-window" => Action::ScreenshotWindow(true, None),
        "switch-preset-column-width" => Action::SwitchPresetColumnWidth,
        "switch-preset-window-height" => Action::SwitchPresetWindowHeight,
        "reset-window-height" => Action::ResetWindowHeight,
        "consume-window-into-column" => Action::ConsumeWindowIntoColumn,
        "expel-window-from-column" => Action::ExpelWindowFromColumn,
        "consume-or-expel-window-left" => Action::ConsumeOrExpelWindowLeft,
        "consume-or-expel-window-right" => Action::ConsumeOrExpelWindowRight,
        "show-hotkey-overlay" => Action::ShowHotkeyOverlay,
        "suspend" => Action::Suspend,
        // Missing actions from roadmap TODO
        "toggle-keyboard-shortcuts-inhibit" => Action::ToggleKeyboardShortcutsInhibit,
        "expand-column-to-available-width" => Action::ExpandColumnToAvailableWidth,
        "center-visible-columns" => Action::CenterVisibleColumns,
        "switch-focus-between-floating-and-tiling" => Action::SwitchFocusBetweenFloatingAndTiling,
        "toggle-column-tabbed-display" => Action::ToggleColumnTabbedDisplay,
        // Additional focus actions
        "focus-column-right-or-first" => Action::FocusColumnRightOrFirst,
        "focus-column-left-or-last" => Action::FocusColumnLeftOrLast,
        "focus-window-or-monitor-up" => Action::FocusWindowOrMonitorUp,
        "focus-window-or-monitor-down" => Action::FocusWindowOrMonitorDown,
        "focus-column-or-monitor-left" => Action::FocusColumnOrMonitorLeft,
        "focus-column-or-monitor-right" => Action::FocusColumnOrMonitorRight,
        "focus-window-down-or-column-left" => Action::FocusWindowDownOrColumnLeft,
        "focus-window-down-or-column-right" => Action::FocusWindowDownOrColumnRight,
        "focus-window-up-or-column-left" => Action::FocusWindowUpOrColumnLeft,
        "focus-window-up-or-column-right" => Action::FocusWindowUpOrColumnRight,
        "focus-window-or-workspace-down" => Action::FocusWindowOrWorkspaceDown,
        "focus-window-or-workspace-up" => Action::FocusWindowOrWorkspaceUp,
        "focus-window-top" => Action::FocusWindowTop,
        "focus-window-bottom" => Action::FocusWindowBottom,
        "focus-window-down-or-top" => Action::FocusWindowDownOrTop,
        "focus-window-up-or-bottom" => Action::FocusWindowUpOrBottom,
        "focus-window-previous" => Action::FocusWindowPrevious,
        "focus-workspace-previous" => Action::FocusWorkspacePrevious,
        // Additional move actions
        "move-column-left-or-to-monitor-left" => Action::MoveColumnLeftOrToMonitorLeft,
        "move-column-right-or-to-monitor-right" => Action::MoveColumnRightOrToMonitorRight,
        "move-window-down-or-to-workspace-down" => Action::MoveWindowDownOrToWorkspaceDown,
        "move-window-up-or-to-workspace-up" => Action::MoveWindowUpOrToWorkspaceUp,
        // Additional monitor actions
        "focus-monitor-previous" => Action::FocusMonitorPrevious,
        "focus-monitor-next" => Action::FocusMonitorNext,
        "move-column-to-monitor-previous" => Action::MoveColumnToMonitorPrevious,
        "move-column-to-monitor-next" => Action::MoveColumnToMonitorNext,
        "move-window-to-monitor-left" => Action::MoveWindowToMonitorLeft,
        "move-window-to-monitor-right" => Action::MoveWindowToMonitorRight,
        "move-window-to-monitor-down" => Action::MoveWindowToMonitorDown,
        "move-window-to-monitor-up" => Action::MoveWindowToMonitorUp,
        "move-window-to-monitor-previous" => Action::MoveWindowToMonitorPrevious,
        "move-window-to-monitor-next" => Action::MoveWindowToMonitorNext,
        "move-workspace-to-monitor-left" => Action::MoveWorkspaceToMonitorLeft,
        "move-workspace-to-monitor-right" => Action::MoveWorkspaceToMonitorRight,
        "move-workspace-to-monitor-down" => Action::MoveWorkspaceToMonitorDown,
        "move-workspace-to-monitor-up" => Action::MoveWorkspaceToMonitorUp,
        "move-workspace-to-monitor-previous" => Action::MoveWorkspaceToMonitorPrevious,
        "move-workspace-to-monitor-next" => Action::MoveWorkspaceToMonitorNext,
        // Column operations
        "swap-window-left" => Action::SwapWindowLeft,
        "swap-window-right" => Action::SwapWindowRight,
        "center-window" => Action::CenterWindow,
        // Size preset actions
        "switch-preset-column-width-back" => Action::SwitchPresetColumnWidthBack,
        "switch-preset-window-height-back" => Action::SwitchPresetWindowHeightBack,
        "maximize-window-to-edges" => Action::MaximizeWindowToEdges,
        // Floating actions
        "move-window-to-floating" => Action::MoveWindowToFloating,
        "move-window-to-tiling" => Action::MoveWindowToTiling,
        "focus-floating" => Action::FocusFloating,
        "focus-tiling" => Action::FocusTiling,
        "toggle-window-rule-opacity" => Action::ToggleWindowRuleOpacity,
        // Debug actions
        "toggle-debug-tint" => Action::ToggleDebugTint,
        "debug-toggle-opaque-regions" => Action::DebugToggleOpaqueRegions,
        "debug-toggle-damage" => Action::DebugToggleDamage,
        // Handle workspace index actions
        "focus-workspace" => {
            if args.is_empty() {
                return None;
            }
            let index: u8 = args[0].parse().ok()?;
            Action::FocusWorkspace(WorkspaceReference::Index(index))
        }
        "move-column-to-workspace" => {
            if args.is_empty() {
                return None;
            }
            let index: u8 = args[0].parse().ok()?;
            Action::MoveColumnToWorkspace(WorkspaceReference::Index(index), true)
        }
        "move-window-to-workspace" => {
            if args.is_empty() {
                return None;
            }
            let index: u8 = args[0].parse().ok()?;
            Action::MoveWindowToWorkspace(WorkspaceReference::Index(index), false)
        }
        "set-column-width" => {
            if args.is_empty() {
                return None;
            }
            let change = parse_size_change(&args[0])?;
            Action::SetColumnWidth(change)
        }
        "set-window-height" => {
            if args.is_empty() {
                return None;
            }
            let change = parse_size_change(&args[0])?;
            Action::SetWindowHeight(change)
        }
        _ => {
            debug!("Unknown action in pending config: {}", action_str);
            return None;
        }
    })
}

/// Convert a JSON value to a WindowRule struct.
fn json_to_window_rule(json: &serde_json::Value) -> Option<niri_config::WindowRule> {
    let obj = json.as_object()?;

    let mut rule = niri_config::WindowRule::default();

    // Helper function to parse a single match object
    let parse_match = |match_obj: &serde_json::Map<String, serde_json::Value>| -> Option<niri_config::window_rule::Match> {
        let app_id = match_obj
            .get("app_id")
            .and_then(|v| v.as_str())
            .and_then(|s| regex::Regex::new(s).ok().map(niri_config::utils::RegexEq));
        let title = match_obj
            .get("title")
            .and_then(|v| v.as_str())
            .and_then(|s| regex::Regex::new(s).ok().map(niri_config::utils::RegexEq));

        if app_id.is_some() || title.is_some() {
            Some(niri_config::window_rule::Match {
                app_id,
                title,
                is_active: None,
                is_active_in_column: None,
                is_focused: None,
                is_floating: None,
                is_urgent: None,
                at_startup: None,
                is_window_cast_target: None,
            })
        } else {
            None
        }
    };

    // Parse matches - support both "matches" (plural, array) and "match" (singular, object)
    if let Some(matches_arr) = obj.get("matches").and_then(|v| v.as_array()) {
        for match_json in matches_arr {
            if let Some(match_obj) = match_json.as_object() {
                if let Some(m) = parse_match(match_obj) {
                    rule.matches.push(m);
                }
            }
        }
    } else if let Some(match_obj) = obj.get("match").and_then(|v| v.as_object()) {
        // Handle singular "match" field (used by niriv2.lua)
        if let Some(m) = parse_match(match_obj) {
            rule.matches.push(m);
        }
    }

    // Parse common rule properties
    if let Some(output) = obj.get("open_on_output").and_then(|v| v.as_str()) {
        rule.open_on_output = Some(output.to_string());
    }
    if let Some(maximized) = obj.get("open_maximized").and_then(|v| v.as_bool()) {
        rule.open_maximized = Some(maximized);
    }
    if let Some(fullscreen) = obj.get("open_fullscreen").and_then(|v| v.as_bool()) {
        rule.open_fullscreen = Some(fullscreen);
    }
    if let Some(floating) = obj.get("open_floating").and_then(|v| v.as_bool()) {
        rule.open_floating = Some(floating);
    }
    if let Some(width) = obj.get("default_column_width") {
        if let Some(size) = json_to_preset_size(width) {
            rule.default_column_width = Some(niri_config::DefaultPresetSize(Some(size)));
        } else if width.is_object() || width.is_null() {
            // Empty object {} means "auto"
            rule.default_column_width = Some(niri_config::DefaultPresetSize(None));
        }
    }

    Some(rule)
}

/// Check if a bind matches the given criteria.
fn bind_matches_criteria(bind: &Bind, criteria: &serde_json::Value) -> bool {
    let obj = match criteria.as_object() {
        Some(o) => o,
        None => return false,
    };

    // Match by key string
    if let Some(key_str) = obj.get("key").and_then(|v| v.as_str()) {
        let bind_key_str = format!("{:?}", bind.key);
        if !bind_key_str.contains(key_str) && key_str != bind_key_str {
            return false;
        }
    }

    // Match by action string
    if let Some(action_str) = obj.get("action").and_then(|v| v.as_str()) {
        let bind_action_str = format!("{:?}", bind.action);
        if !bind_action_str
            .to_lowercase()
            .contains(&action_str.to_lowercase())
        {
            return false;
        }
    }

    true
}

/// Check if a window rule matches the given criteria.
fn window_rule_matches_criteria(
    rule: &niri_config::WindowRule,
    criteria: &serde_json::Value,
) -> bool {
    let obj = match criteria.as_object() {
        Some(o) => o,
        None => return false,
    };

    // Match by app_id in matches
    if let Some(app_id) = obj.get("app_id").and_then(|v| v.as_str()) {
        let has_match = rule.matches.iter().any(|m| {
            m.app_id
                .as_ref()
                .map(|r| r.0.as_str() == app_id)
                .unwrap_or(false)
        });
        if !has_match {
            return false;
        }
    }

    // Match by title in matches
    if let Some(title) = obj.get("title").and_then(|v| v.as_str()) {
        let has_match = rule.matches.iter().any(|m| {
            m.title
                .as_ref()
                .map(|r| r.0.as_str() == title)
                .unwrap_or(false)
        });
        if !has_match {
            return false;
        }
    }

    true
}

/// Apply gestures scalar changes.
fn apply_gestures_scalar_change(
    gestures: &mut niri_config::Gestures,
    path: &[&str],
    value: &serde_json::Value,
) -> bool {
    if path.is_empty() {
        return false;
    }

    match path[0] {
        "dnd_edge_view_scroll" => {
            if path.len() > 1 {
                match path[1] {
                    "trigger_width" => {
                        if let Some(n) = value.as_f64() {
                            gestures.dnd_edge_view_scroll.trigger_width = n;
                            return true;
                        }
                    }
                    "delay_ms" => {
                        if let Some(n) = value.as_u64() {
                            gestures.dnd_edge_view_scroll.delay_ms = n as u16;
                            return true;
                        }
                    }
                    "max_speed" => {
                        if let Some(n) = value.as_f64() {
                            gestures.dnd_edge_view_scroll.max_speed = n;
                            return true;
                        }
                    }
                    _ => {}
                }
            }
        }
        "dnd_edge_workspace_switch" => {
            if path.len() > 1 {
                match path[1] {
                    "trigger_height" => {
                        if let Some(n) = value.as_f64() {
                            gestures.dnd_edge_workspace_switch.trigger_height = n;
                            return true;
                        }
                    }
                    "delay_ms" => {
                        if let Some(n) = value.as_u64() {
                            gestures.dnd_edge_workspace_switch.delay_ms = n as u16;
                            return true;
                        }
                    }
                    "max_speed" => {
                        if let Some(n) = value.as_f64() {
                            gestures.dnd_edge_workspace_switch.max_speed = n;
                            return true;
                        }
                    }
                    _ => {}
                }
            }
        }
        "hot_corners" => {
            if path.len() > 1 && path[1] == "off" {
                if let Some(b) = value.as_bool() {
                    gestures.hot_corners.off = b;
                    return true;
                }
            }
        }
        _ => {}
    }
    false
}

/// Apply overview scalar changes.
fn apply_overview_scalar_change(
    overview: &mut niri_config::Overview,
    path: &[&str],
    value: &serde_json::Value,
) -> bool {
    if path.is_empty() {
        return false;
    }

    match path[0] {
        "zoom" => {
            if let Some(n) = value.as_f64() {
                overview.zoom = n;
                return true;
            } else if let Some(n) = value.as_i64() {
                overview.zoom = n as f64;
                return true;
            }
        }
        "backdrop_color" => {
            if let Some(color) = json_to_color(value) {
                overview.backdrop_color = color;
                return true;
            }
        }
        "workspace_shadow" => {
            if path.len() > 1 {
                match path[1] {
                    "off" => {
                        if let Some(b) = value.as_bool() {
                            overview.workspace_shadow.off = b;
                            return true;
                        }
                    }
                    "softness" => {
                        if let Some(n) = value.as_f64() {
                            overview.workspace_shadow.softness = n;
                            return true;
                        }
                    }
                    "spread" => {
                        if let Some(n) = value.as_f64() {
                            overview.workspace_shadow.spread = n;
                            return true;
                        }
                    }
                    "color" => {
                        if let Some(color) = json_to_color(value) {
                            overview.workspace_shadow.color = color;
                            return true;
                        }
                    }
                    _ => {}
                }
            }
        }
        _ => {}
    }
    false
}

/// Apply recent_windows scalar changes.
fn apply_recent_windows_scalar_change(
    recent_windows: &mut niri_config::RecentWindows,
    path: &[&str],
    value: &serde_json::Value,
) -> bool {
    if path.is_empty() {
        return false;
    }

    match path[0] {
        "off" => {
            if let Some(b) = value.as_bool() {
                recent_windows.on = !b;
                return true;
            }
        }
        "debounce_ms" => {
            if let Some(n) = value.as_u64() {
                recent_windows.debounce_ms = n as u16;
                return true;
            }
        }
        "open_delay_ms" => {
            if let Some(n) = value.as_u64() {
                recent_windows.open_delay_ms = n as u16;
                return true;
            }
        }
        "highlight" => {
            if path.len() > 1 {
                match path[1] {
                    "active_color" => {
                        if let Some(color) = json_to_color(value) {
                            recent_windows.highlight.active_color = color;
                            return true;
                        }
                    }
                    "urgent_color" => {
                        if let Some(color) = json_to_color(value) {
                            recent_windows.highlight.urgent_color = color;
                            return true;
                        }
                    }
                    "padding" => {
                        if let Some(n) = value.as_f64() {
                            recent_windows.highlight.padding = n;
                            return true;
                        }
                    }
                    "corner_radius" => {
                        if let Some(n) = value.as_f64() {
                            recent_windows.highlight.corner_radius = n;
                            return true;
                        }
                    }
                    _ => {}
                }
            }
        }
        "previews" => {
            if path.len() > 1 {
                match path[1] {
                    "max_height" => {
                        if let Some(n) = value.as_f64() {
                            recent_windows.previews.max_height = n;
                            return true;
                        }
                    }
                    "max_scale" => {
                        if let Some(n) = value.as_f64() {
                            recent_windows.previews.max_scale = n;
                            return true;
                        }
                    }
                    _ => {}
                }
            }
        }
        _ => {}
    }
    false
}

/// Apply clipboard scalar changes.
fn apply_clipboard_scalar_change(
    clipboard: &mut niri_config::Clipboard,
    path: &[&str],
    value: &serde_json::Value,
) -> bool {
    if path.is_empty() {
        return false;
    }

    if path[0] == "disable_primary" {
        if let Some(b) = value.as_bool() {
            clipboard.disable_primary = b;
            return true;
        }
    }
    false
}

/// Apply hotkey_overlay scalar changes.
fn apply_hotkey_overlay_scalar_change(
    hotkey_overlay: &mut niri_config::HotkeyOverlay,
    path: &[&str],
    value: &serde_json::Value,
) -> bool {
    if path.is_empty() {
        return false;
    }

    if path[0] == "skip_at_startup" {
        if let Some(b) = value.as_bool() {
            hotkey_overlay.skip_at_startup = b;
            return true;
        }
    }
    false
}

/// Apply config_notification scalar changes.
fn apply_config_notification_scalar_change(
    config_notification: &mut niri_config::ConfigNotification,
    path: &[&str],
    value: &serde_json::Value,
) -> bool {
    if path.is_empty() {
        return false;
    }

    if path[0] == "disable_failed" {
        if let Some(b) = value.as_bool() {
            config_notification.disable_failed = b;
            return true;
        }
    }
    false
}

/// Apply debug scalar changes.
fn apply_debug_scalar_change(
    debug: &mut niri_config::Debug,
    path: &[&str],
    value: &serde_json::Value,
) -> bool {
    if path.is_empty() {
        return false;
    }

    match path[0] {
        "wait_for_frame_completion_before_queueing" => {
            if let Some(b) = value.as_bool() {
                debug.wait_for_frame_completion_before_queueing = b;
                return true;
            }
        }
        "enable_overlay_planes" => {
            if let Some(b) = value.as_bool() {
                debug.enable_overlay_planes = b;
                return true;
            }
        }
        "disable_cursor_plane" => {
            if let Some(b) = value.as_bool() {
                debug.disable_cursor_plane = b;
                return true;
            }
        }
        "render_drm_device" => {
            if let Some(s) = value.as_str() {
                debug.render_drm_device = Some(std::path::PathBuf::from(s));
                return true;
            }
        }
        "emulate_zero_presentation_time" => {
            if let Some(b) = value.as_bool() {
                debug.emulate_zero_presentation_time = b;
                return true;
            }
        }
        _ => {}
    }
    false
}

/// Apply xwayland_satellite scalar changes.
fn apply_xwayland_satellite_scalar_change(
    xwayland_satellite: &mut niri_config::XwaylandSatellite,
    path: &[&str],
    value: &serde_json::Value,
) -> bool {
    if path.is_empty() {
        return false;
    }

    match path[0] {
        "off" => {
            if let Some(b) = value.as_bool() {
                xwayland_satellite.off = b;
                return true;
            }
        }
        "path" => {
            if let Some(s) = value.as_str() {
                xwayland_satellite.path = s.to_string();
                return true;
            }
        }
        _ => {}
    }
    false
}

/// Convert a JSON value to a Color.
fn json_to_color(value: &serde_json::Value) -> Option<niri_config::Color> {
    // Handle string format (e.g., "#ff0000" or "rgba(255, 0, 0, 1.0)")
    if let Some(s) = value.as_str() {
        // Try parsing as hex color
        if let Some(s) = s.strip_prefix('#') {
            let len = s.len();
            if len == 6 || len == 8 {
                let r = u8::from_str_radix(&s[0..2], 16).ok()?;
                let g = u8::from_str_radix(&s[2..4], 16).ok()?;
                let b = u8::from_str_radix(&s[4..6], 16).ok()?;
                let a = if len == 8 {
                    u8::from_str_radix(&s[6..8], 16).ok()?
                } else {
                    255
                };
                return Some(niri_config::Color::from_rgba8_unpremul(r, g, b, a));
            }
        }
        return None;
    }

    // Handle object format with r, g, b, a fields
    if let Some(obj) = value.as_object() {
        let r = obj.get("r").and_then(|v| v.as_u64()).unwrap_or(0) as u8;
        let g = obj.get("g").and_then(|v| v.as_u64()).unwrap_or(0) as u8;
        let b = obj.get("b").and_then(|v| v.as_u64()).unwrap_or(0) as u8;
        let a = obj.get("a").and_then(|v| v.as_u64()).unwrap_or(255) as u8;
        return Some(niri_config::Color::from_rgba8_unpremul(r, g, b, a));
    }

    // Handle array format [r, g, b] or [r, g, b, a]
    if let Some(arr) = value.as_array() {
        if arr.len() >= 3 {
            let r = arr[0].as_u64().unwrap_or(0) as u8;
            let g = arr[1].as_u64().unwrap_or(0) as u8;
            let b = arr[2].as_u64().unwrap_or(0) as u8;
            let a = arr.get(3).and_then(|v| v.as_u64()).unwrap_or(255) as u8;
            return Some(niri_config::Color::from_rgba8_unpremul(r, g, b, a));
        }
    }

    None
}

/// Convert a JSON value to a PresetSize.
fn json_to_preset_size(value: &serde_json::Value) -> Option<niri_config::PresetSize> {
    if let Some(obj) = value.as_object() {
        // Check for proportion (e.g., { proportion = 0.5 })
        if let Some(prop) = obj.get("proportion").and_then(|v| v.as_f64()) {
            return Some(niri_config::PresetSize::Proportion(prop));
        }
        // Check for fixed (e.g., { fixed = 800 })
        if let Some(fixed) = obj.get("fixed").and_then(|v| v.as_i64()) {
            return Some(niri_config::PresetSize::Fixed(fixed as i32));
        }
    }

    // Try as plain number (assume proportion if < 1, fixed otherwise)
    if let Some(n) = value.as_f64() {
        if n <= 1.0 && n > 0.0 {
            return Some(niri_config::PresetSize::Proportion(n));
        } else {
            return Some(niri_config::PresetSize::Fixed(n as i32));
        }
    }

    None
}

/// Convert a JSON value to an Output config.
fn json_to_output(json: &serde_json::Value) -> Option<niri_config::Output> {
    let obj = json.as_object()?;

    let mut output = niri_config::Output::default();

    // Set the output name
    if let Some(name) = obj.get("name").and_then(|v| v.as_str()) {
        output.name = name.to_string();
    }

    if let Some(off) = obj.get("off").and_then(|v| v.as_bool()) {
        output.off = off;
    }

    if let Some(scale) = obj.get("scale").and_then(|v| v.as_f64()) {
        output.scale = Some(FloatOrInt(scale));
    }

    if let Some(vrr) = obj.get("vrr").and_then(|v| v.as_bool()) {
        if vrr {
            output.variable_refresh_rate = Some(niri_config::output::Vrr::default());
        }
    }

    if let Some(pos) = obj.get("position").and_then(|v| v.as_object()) {
        let x = pos.get("x").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
        let y = pos.get("y").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
        output.position = Some(Position { x, y });
    }

    if let Some(mode) = obj.get("mode").and_then(|v| v.as_object()) {
        let width = mode.get("width").and_then(|v| v.as_u64()).unwrap_or(0) as u16;
        let height = mode.get("height").and_then(|v| v.as_u64()).unwrap_or(0) as u16;
        let refresh = mode.get("refresh").and_then(|v| v.as_f64());
        output.mode = Some(niri_config::output::Mode {
            custom: false,
            mode: ConfiguredMode {
                width,
                height,
                refresh,
            },
        });
    }

    if let Some(transform_str) = obj.get("transform").and_then(|v| v.as_str()) {
        output.transform = match transform_str {
            "normal" => Transform::Normal,
            "90" => Transform::_90,
            "180" => Transform::_180,
            "270" => Transform::_270,
            "flipped" => Transform::Flipped,
            "flipped-90" => Transform::Flipped90,
            "flipped-180" => Transform::Flipped180,
            "flipped-270" => Transform::Flipped270,
            _ => Transform::Normal,
        };
    }

    Some(output)
}

/// Convert a JSON value to a SpawnAtStartup config.
fn json_to_spawn(json: &serde_json::Value) -> Option<niri_config::SpawnAtStartup> {
    // Handle string format (simple command)
    if let Some(s) = json.as_str() {
        return Some(niri_config::SpawnAtStartup {
            command: vec![s.to_string()],
        });
    }

    // Handle array format (command with args)
    if let Some(arr) = json.as_array() {
        let command: Vec<String> = arr
            .iter()
            .filter_map(|v| v.as_str().map(String::from))
            .collect();
        if !command.is_empty() {
            return Some(niri_config::SpawnAtStartup { command });
        }
    }

    // Handle object format with command field
    if let Some(obj) = json.as_object() {
        if let Some(cmd) = obj.get("command") {
            if let Some(s) = cmd.as_str() {
                return Some(niri_config::SpawnAtStartup {
                    command: vec![s.to_string()],
                });
            }
            if let Some(arr) = cmd.as_array() {
                let command: Vec<String> = arr
                    .iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect();
                if !command.is_empty() {
                    return Some(niri_config::SpawnAtStartup { command });
                }
            }
        }
    }

    None
}

/// Convert a JSON value to an EnvironmentVariable.
fn json_to_environment(json: &serde_json::Value) -> Option<niri_config::EnvironmentVariable> {
    if let Some(obj) = json.as_object() {
        let name = obj.get("key").and_then(|v| v.as_str())?.to_string();
        let value = obj.get("value").and_then(|v| v.as_str()).map(String::from);
        return Some(niri_config::EnvironmentVariable { name, value });
    }
    None
}

/// Convert a JSON value to a Workspace config.
fn json_to_workspace(json: &serde_json::Value) -> Option<niri_config::Workspace> {
    let obj = json.as_object()?;

    // Name is required
    let name = obj.get("name").and_then(|v| v.as_str())?;
    let open_on_output = obj
        .get("open_on_output")
        .and_then(|v| v.as_str())
        .map(String::from);

    Some(niri_config::Workspace {
        name: WorkspaceName(name.to_string()),
        open_on_output,
        layout: None,
    })
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use parking_lot::Mutex;

    use super::*;
    use crate::config_proxy::PendingConfigChanges;

    /// Helper to create a runtime with config proxy initialized
    fn setup_runtime() -> (LuaRuntime, Arc<Mutex<PendingConfigChanges>>) {
        let mut runtime = LuaRuntime::new().unwrap();
        let shared = runtime.init_empty_config_proxy().unwrap();
        (runtime, shared)
    }

    // ========================================================================
    // apply_pending_lua_config - Basic Tests
    // ========================================================================

    #[test]
    fn apply_pending_empty_changes() {
        let (runtime, _shared) = setup_runtime();
        let mut config = Config::default();
        let count = apply_pending_lua_config(&runtime, &mut config);
        assert_eq!(count, 0);
    }

    #[test]
    fn apply_pending_layout_gaps() {
        let (runtime, _shared) = setup_runtime();
        runtime.load_string("niri.config.layout.gaps = 24").unwrap();

        let mut config = Config::default();
        let count = apply_pending_lua_config(&runtime, &mut config);

        assert!(count > 0);
        assert_eq!(config.layout.gaps, 24.0);
    }

    #[test]
    fn apply_pending_prefer_no_csd() {
        let (runtime, _shared) = setup_runtime();
        runtime
            .load_string("niri.config.prefer_no_csd = true")
            .unwrap();

        let mut config = Config::default();
        let count = apply_pending_lua_config(&runtime, &mut config);

        assert!(count > 0);
        assert!(config.prefer_no_csd);
    }

    // ========================================================================
    // apply_pending_lua_config - Layout Tests
    // ========================================================================

    #[test]
    fn apply_pending_layout_center_focused_column() {
        let (runtime, _shared) = setup_runtime();
        runtime
            .load_string("niri.config.layout.center_focused_column = 'always'")
            .unwrap();

        let mut config = Config::default();
        apply_pending_lua_config(&runtime, &mut config);

        assert_eq!(
            config.layout.center_focused_column,
            niri_config::layout::CenterFocusedColumn::Always
        );
    }

    #[test]
    fn apply_pending_layout_preset_column_widths() {
        let (runtime, _shared) = setup_runtime();
        runtime
            .load_string(
                r#"
            niri.config.layout.preset_column_widths = {
                { proportion = 0.33 },
                { proportion = 0.5 },
                { proportion = 0.67 }
            }
        "#,
            )
            .unwrap();

        let mut config = Config::default();
        apply_pending_lua_config(&runtime, &mut config);

        assert_eq!(config.layout.preset_column_widths.len(), 3);
    }

    #[test]
    fn apply_pending_layout_border_active_color() {
        let (runtime, _shared) = setup_runtime();
        // Set both border.off = false (to enable) and the color
        runtime
            .load_string(
                r#"
            niri.config.layout.border.off = false
            niri.config.layout.border.active.color = '#ff5500'
        "#,
            )
            .unwrap();

        let mut config = Config::default();
        apply_pending_lua_config(&runtime, &mut config);

        // Border should be enabled (off = false)
        assert!(!config.layout.border.off);
    }

    #[test]
    fn apply_pending_layout_focus_ring() {
        let (runtime, _shared) = setup_runtime();
        runtime
            .load_string(
                r#"
            niri.config.layout.focus_ring.active.color = '#00ff00'
            niri.config.layout.focus_ring.width = 4
        "#,
            )
            .unwrap();

        let mut config = Config::default();
        apply_pending_lua_config(&runtime, &mut config);

        assert_eq!(config.layout.focus_ring.width, 4.0);
    }

    // ========================================================================
    // apply_pending_lua_config - Collection Tests (binds)
    // ========================================================================

    #[test]
    fn apply_pending_binds_spawn() {
        let (runtime, _shared) = setup_runtime();
        runtime
            .load_string(
                r#"
            niri.config.binds:add({
                key = "Mod+Return",
                action = "spawn",
                args = { "alacritty" }
            })
        "#,
            )
            .unwrap();

        let mut config = Config::default();
        let initial_binds = config.binds.0.len();
        apply_pending_lua_config(&runtime, &mut config);

        assert_eq!(config.binds.0.len(), initial_binds + 1);
    }

    #[test]
    fn apply_pending_binds_focus_workspace() {
        let (runtime, _shared) = setup_runtime();
        runtime
            .load_string(
                r#"
            niri.config.binds:add({
                key = "Mod+1",
                action = "focus-workspace",
                args = { 1 }
            })
        "#,
            )
            .unwrap();

        let mut config = Config::default();
        let initial_binds = config.binds.0.len();
        apply_pending_lua_config(&runtime, &mut config);

        assert_eq!(config.binds.0.len(), initial_binds + 1);
    }

    #[test]
    fn apply_pending_binds_multiple() {
        let (runtime, _shared) = setup_runtime();
        runtime
            .load_string(
                r#"
            niri.config.binds:add({ key = "Mod+1", action = "focus-workspace", args = { 1 } })
            niri.config.binds:add({ key = "Mod+2", action = "focus-workspace", args = { 2 } })
            niri.config.binds:add({ key = "Mod+3", action = "focus-workspace", args = { 3 } })
        "#,
            )
            .unwrap();

        let mut config = Config::default();
        let initial_binds = config.binds.0.len();
        apply_pending_lua_config(&runtime, &mut config);

        assert_eq!(config.binds.0.len(), initial_binds + 3);
    }

    // ========================================================================
    // apply_pending_lua_config - Collection Tests (spawn_at_startup)
    // ========================================================================

    #[test]
    fn apply_pending_spawn_at_startup() {
        let (runtime, _shared) = setup_runtime();
        runtime
            .load_string(
                r#"
            niri.config.spawn_at_startup:add({
                command = { "waybar" }
            })
        "#,
            )
            .unwrap();

        let mut config = Config::default();
        let initial_spawns = config.spawn_at_startup.len();
        apply_pending_lua_config(&runtime, &mut config);

        assert_eq!(config.spawn_at_startup.len(), initial_spawns + 1);
    }

    #[test]
    fn apply_pending_spawn_at_startup_with_args() {
        let (runtime, _shared) = setup_runtime();
        runtime
            .load_string(
                r#"
            niri.config.spawn_at_startup:add({
                command = { "swaybg", "-i", "/path/to/image.png" }
            })
        "#,
            )
            .unwrap();

        let mut config = Config::default();
        apply_pending_lua_config(&runtime, &mut config);

        assert!(!config.spawn_at_startup.is_empty());
    }

    // ========================================================================
    // apply_pending_lua_config - Collection Tests (workspaces)
    // ========================================================================

    #[test]
    fn apply_pending_workspaces() {
        let (runtime, _shared) = setup_runtime();
        runtime
            .load_string(
                r#"
            niri.config.workspaces:add({
                name = "main"
            })
        "#,
            )
            .unwrap();

        let mut config = Config::default();
        let initial_ws = config.workspaces.len();
        apply_pending_lua_config(&runtime, &mut config);

        assert_eq!(config.workspaces.len(), initial_ws + 1);
    }

    #[test]
    fn apply_pending_workspaces_with_output() {
        let (runtime, _shared) = setup_runtime();
        runtime
            .load_string(
                r#"
            niri.config.workspaces:add({
                name = "work",
                open_on_output = "eDP-1"
            })
        "#,
            )
            .unwrap();

        let mut config = Config::default();
        apply_pending_lua_config(&runtime, &mut config);

        let ws = config.workspaces.iter().find(|w| w.name.0 == "work");
        assert!(ws.is_some());
        assert_eq!(ws.unwrap().open_on_output, Some("eDP-1".to_string()));
    }

    // ========================================================================
    // apply_pending_lua_config - Collection Tests (window_rules)
    // ========================================================================

    #[test]
    fn apply_pending_window_rules() {
        let (runtime, _shared) = setup_runtime();
        runtime
            .load_string(
                r#"
            niri.config.window_rules:add({
                matches = { { app_id = "firefox" } },
                default_column_width = { proportion = 0.5 }
            })
        "#,
            )
            .unwrap();

        let mut config = Config::default();
        let initial_rules = config.window_rules.len();
        apply_pending_lua_config(&runtime, &mut config);

        assert_eq!(config.window_rules.len(), initial_rules + 1);
    }

    // ========================================================================
    // apply_pending_lua_config - Animations Tests
    // ========================================================================

    #[test]
    fn apply_pending_animations_off() {
        let (runtime, _shared) = setup_runtime();
        runtime
            .load_string("niri.config.animations.off = true")
            .unwrap();

        let mut config = Config::default();
        apply_pending_lua_config(&runtime, &mut config);

        assert!(config.animations.off);
    }

    #[test]
    fn apply_pending_animations_slowdown() {
        let (runtime, _shared) = setup_runtime();
        runtime
            .load_string("niri.config.animations.slowdown = 2.5")
            .unwrap();

        let mut config = Config::default();
        apply_pending_lua_config(&runtime, &mut config);

        assert_eq!(config.animations.slowdown, 2.5);
    }

    // ========================================================================
    // apply_pending_lua_config - Input Tests
    // ========================================================================

    #[test]
    fn apply_pending_input_keyboard_repeat_delay() {
        let (runtime, _shared) = setup_runtime();
        runtime
            .load_string("niri.config.input.keyboard.repeat_delay = 300")
            .unwrap();

        let mut config = Config::default();
        apply_pending_lua_config(&runtime, &mut config);

        assert_eq!(config.input.keyboard.repeat_delay, 300);
    }

    #[test]
    fn apply_pending_input_keyboard_repeat_rate() {
        let (runtime, _shared) = setup_runtime();
        runtime
            .load_string("niri.config.input.keyboard.repeat_rate = 50")
            .unwrap();

        let mut config = Config::default();
        apply_pending_lua_config(&runtime, &mut config);

        assert_eq!(config.input.keyboard.repeat_rate, 50);
    }

    #[test]
    fn apply_pending_input_touchpad_tap() {
        let (runtime, _shared) = setup_runtime();
        runtime
            .load_string("niri.config.input.touchpad.tap = true")
            .unwrap();

        let mut config = Config::default();
        apply_pending_lua_config(&runtime, &mut config);

        assert!(config.input.touchpad.tap);
    }

    #[test]
    fn apply_pending_input_touchpad_natural_scroll() {
        let (runtime, _shared) = setup_runtime();
        runtime
            .load_string("niri.config.input.touchpad.natural_scroll = true")
            .unwrap();

        let mut config = Config::default();
        apply_pending_lua_config(&runtime, &mut config);

        assert!(config.input.touchpad.natural_scroll);
    }

    #[test]
    fn apply_pending_input_mouse_accel_speed() {
        let (runtime, _shared) = setup_runtime();
        runtime
            .load_string("niri.config.input.mouse.accel_speed = 0.5")
            .unwrap();

        let mut config = Config::default();
        apply_pending_lua_config(&runtime, &mut config);

        // accel_speed is FloatOrInt, compare the float value
        assert_eq!(config.input.mouse.accel_speed.0, 0.5);
    }

    // ========================================================================
    // json_to_* Helper Function Tests
    // ========================================================================

    #[test]
    fn json_to_color_hex_string() {
        let json = serde_json::json!("#ff5500");
        let result = json_to_color(&json);
        assert!(result.is_some());
    }

    #[test]
    fn json_to_color_rgb_object() {
        let json = serde_json::json!({ "r": 255, "g": 128, "b": 0, "a": 255 });
        let result = json_to_color(&json);
        assert!(result.is_some());
    }

    #[test]
    fn json_to_preset_size_proportion() {
        let json = serde_json::json!({ "proportion": 0.5 });
        let result = json_to_preset_size(&json);
        assert!(result.is_some());
    }

    #[test]
    fn json_to_preset_size_fixed() {
        let json = serde_json::json!({ "fixed": 800 });
        let result = json_to_preset_size(&json);
        assert!(result.is_some());
    }

    #[test]
    fn json_to_bind_spawn() {
        let json = serde_json::json!({
            "key": "Mod+Return",
            "action": "spawn",
            "args": ["alacritty"]
        });
        let result = json_to_bind(&json);
        assert!(result.is_some());
    }

    #[test]
    fn json_to_bind_focus_workspace() {
        let json = serde_json::json!({
            "key": "Mod+1",
            "action": "focus-workspace",
            "args": [1]
        });
        let result = json_to_bind(&json);
        assert!(result.is_some());
    }

    #[test]
    fn json_to_spawn_simple() {
        let json = serde_json::json!({
            "command": ["waybar"]
        });
        let result = json_to_spawn(&json);
        assert!(result.is_some());
    }

    #[test]
    fn json_to_workspace_with_name() {
        let json = serde_json::json!({
            "name": "main"
        });
        let result = json_to_workspace(&json);
        assert!(result.is_some());
        assert_eq!(result.unwrap().name.0, "main");
    }

    // ========================================================================
    // parse_action_from_str - New Action Mappings Tests
    // ========================================================================

    #[test]
    fn parse_action_toggle_keyboard_shortcuts_inhibit() {
        let json = serde_json::json!({
            "key": "Mod+Escape",
            "action": "toggle-keyboard-shortcuts-inhibit"
        });
        let result = json_to_bind(&json);
        assert!(result.is_some());
        let bind = result.unwrap();
        assert!(matches!(
            bind.action,
            Action::ToggleKeyboardShortcutsInhibit
        ));
    }

    #[test]
    fn parse_action_expand_column_to_available_width() {
        let json = serde_json::json!({
            "key": "Mod+E",
            "action": "expand-column-to-available-width"
        });
        let result = json_to_bind(&json);
        assert!(result.is_some());
        let bind = result.unwrap();
        assert!(matches!(bind.action, Action::ExpandColumnToAvailableWidth));
    }

    #[test]
    fn parse_action_center_visible_columns() {
        let json = serde_json::json!({
            "key": "Mod+C",
            "action": "center-visible-columns"
        });
        let result = json_to_bind(&json);
        assert!(result.is_some());
        let bind = result.unwrap();
        assert!(matches!(bind.action, Action::CenterVisibleColumns));
    }

    #[test]
    fn parse_action_switch_focus_floating_tiling() {
        let json = serde_json::json!({
            "key": "Mod+Space",
            "action": "switch-focus-between-floating-and-tiling"
        });
        let result = json_to_bind(&json);
        assert!(result.is_some());
        let bind = result.unwrap();
        assert!(matches!(
            bind.action,
            Action::SwitchFocusBetweenFloatingAndTiling
        ));
    }

    #[test]
    fn parse_action_toggle_column_tabbed_display() {
        let json = serde_json::json!({
            "key": "Mod+T",
            "action": "toggle-column-tabbed-display"
        });
        let result = json_to_bind(&json);
        assert!(result.is_some());
        let bind = result.unwrap();
        assert!(matches!(bind.action, Action::ToggleColumnTabbedDisplay));
    }

    #[test]
    fn parse_action_focus_window_previous() {
        let json = serde_json::json!({
            "key": "Mod+Tab",
            "action": "focus-window-previous"
        });
        let result = json_to_bind(&json);
        assert!(result.is_some());
        let bind = result.unwrap();
        assert!(matches!(bind.action, Action::FocusWindowPrevious));
    }

    #[test]
    fn parse_action_toggle_debug_tint() {
        let json = serde_json::json!({
            "key": "Mod+Shift+D",
            "action": "toggle-debug-tint"
        });
        let result = json_to_bind(&json);
        assert!(result.is_some());
        let bind = result.unwrap();
        assert!(matches!(bind.action, Action::ToggleDebugTint));
    }

    #[test]
    fn parse_action_move_window_to_floating() {
        let json = serde_json::json!({
            "key": "Mod+V",
            "action": "move-window-to-floating"
        });
        let result = json_to_bind(&json);
        assert!(result.is_some());
        let bind = result.unwrap();
        assert!(matches!(bind.action, Action::MoveWindowToFloating));
    }
}
