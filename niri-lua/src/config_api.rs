//! Configuration API module for exposing Niri settings to Lua.
//!
//! This module provides the `niri.config` API that allows Lua scripts to read and configure Niri settings.
//! Implements Tier 2 configuration parity with KDL.

use mlua::prelude::*;
use niri_config::{
    Animations, Config, Cursor, Gestures, Input, Layout, Outputs, Overview, Debug, Clipboard,
    HotkeyOverlay, ConfigNotification, XwaylandSatellite,
    animations::{Kind, EasingParams, Curve, SpringParams},
};

/// Main configuration API handler
pub struct ConfigApi;

impl ConfigApi {
    /// Register the configuration API to Lua
    pub fn register_to_lua(lua: &Lua, config: &Config) -> LuaResult<()> {
        let globals = lua.globals();

        // Get or create the niri table
        let niri_table: LuaTable = globals
            .get("niri")
            .unwrap_or_else(|_| lua.create_table().unwrap());

        // Create the config table
        let config_table = lua.create_table()?;

        // Register configuration subsystems
        Self::register_animations(lua, &config_table, &config.animations)?;
        Self::register_input(lua, &config_table, &config.input)?;
        Self::register_layout(lua, &config_table, &config.layout)?;
        Self::register_cursor(lua, &config_table, &config.cursor)?;
        Self::register_output(lua, &config_table, &config.outputs)?;
        Self::register_gestures(lua, &config_table, &config.gestures)?;
        Self::register_overview(lua, &config_table, &config.overview)?;
        Self::register_debug(lua, &config_table, &config.debug)?;
        Self::register_clipboard(lua, &config_table, &config.clipboard)?;
        Self::register_hotkey_overlay(lua, &config_table, &config.hotkey_overlay)?;
        Self::register_config_notification(lua, &config_table, &config.config_notification)?;
        Self::register_xwayland_satellite(lua, &config_table, &config.xwayland_satellite)?;
        Self::register_miscellaneous(lua, &config_table, config)?;

        // Set niri.config
        niri_table.set("config", config_table)?;
        globals.set("niri", niri_table)?;

        Ok(())
    }

    /// Register animations configuration
    fn register_animations(lua: &Lua, config_table: &LuaTable, anim_config: &Animations) -> LuaResult<()> {
        let animations = lua.create_table()?;

        // Global animation settings
        animations.set("off", anim_config.off)?;
        animations.set("slowdown", anim_config.slowdown)?;

        // Workspace switch animation
        let ws_switch = lua.create_table()?;
        Self::set_animation_values(lua, &ws_switch, &anim_config.workspace_switch.0)?;
        animations.set("workspace_switch", ws_switch)?;

        // Window open animation
        let win_open = lua.create_table()?;
        Self::set_animation_values(lua, &win_open, &anim_config.window_open.anim)?;
        if let Some(shader) = &anim_config.window_open.custom_shader {
            win_open.set("custom_shader", shader.clone())?;
        }
        animations.set("window_open", win_open)?;

        // Window close animation
        let win_close = lua.create_table()?;
        Self::set_animation_values(lua, &win_close, &anim_config.window_close.anim)?;
        if let Some(shader) = &anim_config.window_close.custom_shader {
            win_close.set("custom_shader", shader.clone())?;
        }
        animations.set("window_close", win_close)?;

        // Horizontal view movement animation
        let h_view = lua.create_table()?;
        Self::set_animation_values(lua, &h_view, &anim_config.horizontal_view_movement.0)?;
        animations.set("horizontal_view_movement", h_view)?;

        // Window movement animation
        let win_move = lua.create_table()?;
        Self::set_animation_values(lua, &win_move, &anim_config.window_movement.0)?;
        animations.set("window_movement", win_move)?;

        // Window resize animation
        let win_resize = lua.create_table()?;
        Self::set_animation_values(lua, &win_resize, &anim_config.window_resize.anim)?;
        if let Some(shader) = &anim_config.window_resize.custom_shader {
            win_resize.set("custom_shader", shader.clone())?;
        }
        animations.set("window_resize", win_resize)?;

        // Config notification animation
        let cfg_notif = lua.create_table()?;
        Self::set_animation_values(lua, &cfg_notif, &anim_config.config_notification_open_close.0)?;
        animations.set("config_notification_open_close", cfg_notif)?;

        // Exit confirmation animation
        let exit_confirm = lua.create_table()?;
        Self::set_animation_values(lua, &exit_confirm, &anim_config.exit_confirmation_open_close.0)?;
        animations.set("exit_confirmation_open_close", exit_confirm)?;

        // Screenshot UI animation
        let screenshot = lua.create_table()?;
        Self::set_animation_values(lua, &screenshot, &anim_config.screenshot_ui_open.0)?;
        animations.set("screenshot_ui_open", screenshot)?;

        // Overview animation
        let overview = lua.create_table()?;
        Self::set_animation_values(lua, &overview, &anim_config.overview_open_close.0)?;
        animations.set("overview_open_close", overview)?;

        config_table.set("animations", animations)?;
        Ok(())
    }

    /// Helper to set animation values (off flag and animation kind)
    fn set_animation_values(lua: &Lua, table: &LuaTable, anim: &niri_config::Animation) -> LuaResult<()> {
        table.set("off", anim.off)?;

        match anim.kind {
            Kind::Easing(EasingParams { duration_ms, curve }) => {
                table.set("duration_ms", duration_ms)?;
                let curve_str = match curve {
                    Curve::Linear => "linear",
                    Curve::EaseOutQuad => "ease-out-quad",
                    Curve::EaseOutCubic => "ease-out-cubic",
                    Curve::EaseOutExpo => "ease-out-expo",
                    Curve::CubicBezier(x1, y1, x2, y2) => {
                        let bezier_table = lua.create_table()?;
                        bezier_table.set("x1", x1)?;
                        bezier_table.set("y1", y1)?;
                        bezier_table.set("x2", x2)?;
                        bezier_table.set("y2", y2)?;
                        table.set("bezier", bezier_table)?;
                        "cubic-bezier"
                    }
                };
                table.set("curve", curve_str)?;
            }
            Kind::Spring(SpringParams {
                damping_ratio,
                stiffness,
                epsilon,
            }) => {
                table.set("damping_ratio", damping_ratio)?;
                table.set("stiffness", stiffness)?;
                table.set("epsilon", epsilon)?;
                table.set("kind", "spring")?;
            }
        }

        Ok(())
    }

    /// Register input configuration
    fn register_input(lua: &Lua, config_table: &LuaTable, input_config: &Input) -> LuaResult<()> {
        let input = lua.create_table()?;

        // Keyboard settings
        let keyboard = lua.create_table()?;
        keyboard.set("repeat_delay", input_config.keyboard.repeat_delay)?;
        keyboard.set("repeat_rate", input_config.keyboard.repeat_rate)?;
        
        let xkb = lua.create_table()?;
        xkb.set("layout", input_config.keyboard.xkb.layout.clone())?;
        xkb.set("variant", input_config.keyboard.xkb.variant.clone())?;
        xkb.set("rules", input_config.keyboard.xkb.rules.clone())?;
        xkb.set("model", input_config.keyboard.xkb.model.clone())?;
        if let Some(opts) = &input_config.keyboard.xkb.options {
            xkb.set("options", opts.clone())?;
        }
        keyboard.set("xkb", xkb)?;
        keyboard.set("numlock", input_config.keyboard.numlock)?;
        input.set("keyboard", keyboard)?;

        // Mouse settings
        let mouse = lua.create_table()?;
        mouse.set("accel_speed", input_config.mouse.accel_speed.0)?;
        mouse.set("accel_profile", format!("{:?}", input_config.mouse.accel_profile))?;
        input.set("mouse", mouse)?;

        // Touchpad settings
        let touchpad = lua.create_table()?;
        touchpad.set("accel_speed", input_config.touchpad.accel_speed.0)?;
        touchpad.set("accel_profile", format!("{:?}", input_config.touchpad.accel_profile))?;
        touchpad.set("tap", input_config.touchpad.tap)?;
        touchpad.set("tap_button_map", format!("{:?}", input_config.touchpad.tap_button_map))?;
        touchpad.set("natural_scroll", input_config.touchpad.natural_scroll)?;
        input.set("touchpad", touchpad)?;

        // Trackpoint settings
        let trackpoint = lua.create_table()?;
        trackpoint.set("accel_speed", input_config.trackpoint.accel_speed.0)?;
        trackpoint.set("accel_profile", format!("{:?}", input_config.trackpoint.accel_profile))?;
        trackpoint.set("natural_scroll", input_config.trackpoint.natural_scroll)?;
        input.set("trackpoint", trackpoint)?;

        // Global input options
        if let Some(mode) = &input_config.warp_mouse_to_focus {
            if let Some(m) = &mode.mode {
                input.set("warp_mouse_to_focus", format!("{:?}", m).to_lowercase())?;
            }
        }
        if let Some(ffs) = &input_config.focus_follows_mouse {
            let ffs_table = lua.create_table()?;
            if let Some(max_scroll) = ffs.max_scroll_amount {
                ffs_table.set("max_scroll_amount", max_scroll.0)?;
            }
            input.set("focus_follows_mouse", ffs_table)?;
        }

        config_table.set("input", input)?;
        Ok(())
    }

    /// Register layout configuration
    fn register_layout(lua: &Lua, config_table: &LuaTable, layout_config: &Layout) -> LuaResult<()> {
        let layout = lua.create_table()?;

        layout.set("gaps", layout_config.gaps)?;

        // Struts configuration
        let struts = lua.create_table()?;
        struts.set("left", layout_config.struts.left.0)?;
        struts.set("right", layout_config.struts.right.0)?;
        struts.set("top", layout_config.struts.top.0)?;
        struts.set("bottom", layout_config.struts.bottom.0)?;
        layout.set("struts", struts)?;

        // Focus ring configuration
        let focus_ring = lua.create_table()?;
        focus_ring.set("off", layout_config.focus_ring.off)?;
        focus_ring.set("width", layout_config.focus_ring.width)?;
        focus_ring.set("active_color", Self::color_to_hex(&layout_config.focus_ring.active_color))?;
        focus_ring.set("inactive_color", Self::color_to_hex(&layout_config.focus_ring.inactive_color))?;
        focus_ring.set("urgent_color", Self::color_to_hex(&layout_config.focus_ring.urgent_color))?;
        layout.set("focus_ring", focus_ring)?;

        // Border configuration
        let border = lua.create_table()?;
        border.set("off", layout_config.border.off)?;
        border.set("width", layout_config.border.width)?;
        border.set("active_color", Self::color_to_hex(&layout_config.border.active_color))?;
        border.set("inactive_color", Self::color_to_hex(&layout_config.border.inactive_color))?;
        border.set("urgent_color", Self::color_to_hex(&layout_config.border.urgent_color))?;
        layout.set("border", border)?;

        // Shadow configuration
        let shadow = lua.create_table()?;
        shadow.set("on", layout_config.shadow.on)?;
        shadow.set("softness", layout_config.shadow.softness)?;
        shadow.set("spread", layout_config.shadow.spread)?;
        let offset = lua.create_table()?;
        offset.set("x", layout_config.shadow.offset.x.0)?;
        offset.set("y", layout_config.shadow.offset.y.0)?;
        shadow.set("offset", offset)?;
        shadow.set("color", Self::color_to_hex(&layout_config.shadow.color))?;
        shadow.set("draw_behind_window", layout_config.shadow.draw_behind_window)?;
        layout.set("shadow", shadow)?;

        // Tab indicator configuration
        let tab_indicator = lua.create_table()?;
        tab_indicator.set("off", layout_config.tab_indicator.off)?;
        tab_indicator.set("width", layout_config.tab_indicator.width)?;
        if let Some(color) = &layout_config.tab_indicator.active_color {
            tab_indicator.set("active_color", Self::color_to_hex(color))?;
        }
        if let Some(color) = &layout_config.tab_indicator.inactive_color {
            tab_indicator.set("inactive_color", Self::color_to_hex(color))?;
        }
        if let Some(color) = &layout_config.tab_indicator.urgent_color {
            tab_indicator.set("urgent_color", Self::color_to_hex(color))?;
        }
        layout.set("tab_indicator", tab_indicator)?;

        // Insert hint configuration
        let insert_hint = lua.create_table()?;
        insert_hint.set("off", layout_config.insert_hint.off)?;
        insert_hint.set("color", Self::color_to_hex(&layout_config.insert_hint.color))?;
        layout.set("insert_hint", insert_hint)?;

        // Column and window settings
        layout.set("center_focused_column", format!("{:?}", layout_config.center_focused_column).to_lowercase())?;
        layout.set("always_center_single_column", layout_config.always_center_single_column)?;
        layout.set("empty_workspace_above_first", layout_config.empty_workspace_above_first)?;
        layout.set("default_column_display", format!("{:?}", layout_config.default_column_display).to_lowercase())?;
        
        // Preset column widths
        let preset_widths = lua.create_table()?;
        for (i, size) in layout_config.preset_column_widths.iter().enumerate() {
            preset_widths.set(i + 1, Self::preset_size_to_lua_string(size))?;
        }
        layout.set("preset_column_widths", preset_widths)?;

        // Default column width
        if let Some(default_width) = layout_config.default_column_width {
            layout.set("default_column_width", Self::preset_size_to_lua_string(&default_width))?;
        }

        // Preset window heights
        let preset_heights = lua.create_table()?;
        for (i, size) in layout_config.preset_window_heights.iter().enumerate() {
            preset_heights.set(i + 1, Self::preset_size_to_lua_string(size))?;
        }
        layout.set("preset_window_heights", preset_heights)?;

        layout.set("background_color", Self::color_to_hex(&layout_config.background_color))?;

        config_table.set("layout", layout)?;
        Ok(())
    }

    /// Register cursor configuration
    fn register_cursor(lua: &Lua, config_table: &LuaTable, cursor_config: &Cursor) -> LuaResult<()> {
        let cursor = lua.create_table()?;

        cursor.set("xcursor_theme", cursor_config.xcursor_theme.clone())?;
        cursor.set("xcursor_size", cursor_config.xcursor_size)?;
        cursor.set("hide_when_typing", cursor_config.hide_when_typing)?;
        if let Some(ms) = cursor_config.hide_after_inactive_ms {
            cursor.set("hide_after_inactive_ms", ms)?;
        }

        config_table.set("cursor", cursor)?;
        Ok(())
    }

    /// Register output configuration
    fn register_output(lua: &Lua, config_table: &LuaTable, outputs: &Outputs) -> LuaResult<()> {
        let output_table = lua.create_table()?;

        for output in &outputs.0 {
            let output_config = lua.create_table()?;
            
            // Set output name and basic properties
            output_config.set("off", output.off)?;
            
            if let Some(scale) = output.scale {
                output_config.set("scale", scale.0)?;
            }
            
            if let Some(position) = &output.position {
                output_config.set("x", position.x)?;
                output_config.set("y", position.y)?;
            }
            
            if let Some(mode) = &output.mode {
                output_config.set("mode_custom", mode.custom)?;
            }

            output_table.set(output.name.clone(), output_config)?;
        }

        config_table.set("output", output_table)?;
        Ok(())
    }

    /// Register gestures configuration
    fn register_gestures(lua: &Lua, config_table: &LuaTable, gestures_config: &Gestures) -> LuaResult<()> {
        let gestures = lua.create_table()?;

        // Drag and drop edge view scroll
        let dnd_edge_view_scroll = lua.create_table()?;
        dnd_edge_view_scroll.set("trigger_width", gestures_config.dnd_edge_view_scroll.trigger_width)?;
        dnd_edge_view_scroll.set("delay_ms", gestures_config.dnd_edge_view_scroll.delay_ms)?;
        dnd_edge_view_scroll.set("max_speed", gestures_config.dnd_edge_view_scroll.max_speed)?;
        gestures.set("dnd_edge_view_scroll", dnd_edge_view_scroll)?;

        // Drag and drop edge workspace switch
        let dnd_edge_ws_switch = lua.create_table()?;
        dnd_edge_ws_switch.set("trigger_height", gestures_config.dnd_edge_workspace_switch.trigger_height)?;
        dnd_edge_ws_switch.set("delay_ms", gestures_config.dnd_edge_workspace_switch.delay_ms)?;
        dnd_edge_ws_switch.set("max_speed", gestures_config.dnd_edge_workspace_switch.max_speed)?;
        gestures.set("dnd_edge_workspace_switch", dnd_edge_ws_switch)?;

        // Hot corners
        let hot_corners = lua.create_table()?;
        hot_corners.set("off", gestures_config.hot_corners.off)?;
        hot_corners.set("top_left", gestures_config.hot_corners.top_left)?;
        hot_corners.set("top_right", gestures_config.hot_corners.top_right)?;
        hot_corners.set("bottom_left", gestures_config.hot_corners.bottom_left)?;
        hot_corners.set("bottom_right", gestures_config.hot_corners.bottom_right)?;
        gestures.set("hot_corners", hot_corners)?;

        config_table.set("gestures", gestures)?;
        Ok(())
    }

    /// Register overview configuration
    fn register_overview(lua: &Lua, config_table: &LuaTable, overview_config: &Overview) -> LuaResult<()> {
        let overview = lua.create_table()?;
        
        overview.set("zoom", overview_config.zoom)?;
        overview.set("backdrop_color", Self::color_to_hex_noalpha(&overview_config.backdrop_color))?;
        
        // Workspace shadow configuration
        let ws_shadow = lua.create_table()?;
        ws_shadow.set("off", overview_config.workspace_shadow.off)?;
        ws_shadow.set("softness", overview_config.workspace_shadow.softness)?;
        ws_shadow.set("spread", overview_config.workspace_shadow.spread)?;
        let shadow_offset = lua.create_table()?;
        shadow_offset.set("x", overview_config.workspace_shadow.offset.x.0)?;
        shadow_offset.set("y", overview_config.workspace_shadow.offset.y.0)?;
        ws_shadow.set("offset", shadow_offset)?;
        ws_shadow.set("color", Self::color_to_hex(&overview_config.workspace_shadow.color))?;
        overview.set("workspace_shadow", ws_shadow)?;
        
        config_table.set("overview", overview)?;
        Ok(())
    }

    /// Register debug configuration
    fn register_debug(lua: &Lua, config_table: &LuaTable, debug_config: &Debug) -> LuaResult<()> {
        let debug = lua.create_table()?;
        
        if let Some(preview) = &debug_config.preview_render {
            debug.set("preview_render", format!("{:?}", preview))?;
        }
        debug.set("dbus_interfaces_in_non_session_instances", debug_config.dbus_interfaces_in_non_session_instances)?;
        debug.set("wait_for_frame_completion_before_queueing", debug_config.wait_for_frame_completion_before_queueing)?;
        debug.set("enable_overlay_planes", debug_config.enable_overlay_planes)?;
        debug.set("disable_cursor_plane", debug_config.disable_cursor_plane)?;
        debug.set("disable_direct_scanout", debug_config.disable_direct_scanout)?;
        debug.set("keep_max_bpc_unchanged", debug_config.keep_max_bpc_unchanged)?;
        debug.set("restrict_primary_scanout_to_matching_format", debug_config.restrict_primary_scanout_to_matching_format)?;
        if let Some(device) = &debug_config.render_drm_device {
            debug.set("render_drm_device", device.to_string_lossy().to_string())?;
        }
        debug.set("force_pipewire_invalid_modifier", debug_config.force_pipewire_invalid_modifier)?;
        debug.set("emulate_zero_presentation_time", debug_config.emulate_zero_presentation_time)?;
        debug.set("disable_resize_throttling", debug_config.disable_resize_throttling)?;
        debug.set("disable_transactions", debug_config.disable_transactions)?;
        debug.set("keep_laptop_panel_on_when_lid_is_closed", debug_config.keep_laptop_panel_on_when_lid_is_closed)?;
        debug.set("disable_monitor_names", debug_config.disable_monitor_names)?;
        debug.set("strict_new_window_focus_policy", debug_config.strict_new_window_focus_policy)?;
        debug.set("honor_xdg_activation_with_invalid_serial", debug_config.honor_xdg_activation_with_invalid_serial)?;
        debug.set("deactivate_unfocused_windows", debug_config.deactivate_unfocused_windows)?;
        debug.set("skip_cursor_only_updates_during_vrr", debug_config.skip_cursor_only_updates_during_vrr)?;
        
        // Ignored DRM devices
        let ignored_devices = lua.create_table()?;
        for (i, device) in debug_config.ignored_drm_devices.iter().enumerate() {
            ignored_devices.set(i + 1, device.to_string_lossy().to_string())?;
        }
        debug.set("ignored_drm_devices", ignored_devices)?;
        
        config_table.set("debug", debug)?;
        Ok(())
    }

    /// Register clipboard configuration
    fn register_clipboard(lua: &Lua, config_table: &LuaTable, clipboard_config: &Clipboard) -> LuaResult<()> {
        let clipboard = lua.create_table()?;
        clipboard.set("disable_primary", clipboard_config.disable_primary)?;
        config_table.set("clipboard", clipboard)?;
        Ok(())
    }

    /// Register hotkey overlay configuration
    fn register_hotkey_overlay(lua: &Lua, config_table: &LuaTable, hotkey_config: &HotkeyOverlay) -> LuaResult<()> {
        let hotkey = lua.create_table()?;
        hotkey.set("skip_at_startup", hotkey_config.skip_at_startup)?;
        hotkey.set("hide_not_bound", hotkey_config.hide_not_bound)?;
        config_table.set("hotkey_overlay", hotkey)?;
        Ok(())
    }

    /// Register config notification configuration
    fn register_config_notification(lua: &Lua, config_table: &LuaTable, notif_config: &ConfigNotification) -> LuaResult<()> {
        let notif = lua.create_table()?;
        notif.set("disable_failed", notif_config.disable_failed)?;
        config_table.set("config_notification", notif)?;
        Ok(())
    }

    /// Register xwayland satellite configuration
    fn register_xwayland_satellite(lua: &Lua, config_table: &LuaTable, xwayland_config: &XwaylandSatellite) -> LuaResult<()> {
        let xwayland = lua.create_table()?;
        xwayland.set("off", xwayland_config.off)?;
        xwayland.set("path", xwayland_config.path.clone())?;
        config_table.set("xwayland_satellite", xwayland)?;
        Ok(())
    }

    /// Register miscellaneous configuration
    fn register_miscellaneous(lua: &Lua, config_table: &LuaTable, config: &Config) -> LuaResult<()> {
        // Spawn at startup commands
        let spawn_at_startup = lua.create_table()?;
        for (i, spawn) in config.spawn_at_startup.iter().enumerate() {
            let cmd_table = lua.create_table()?;
            for (j, arg) in spawn.command.iter().enumerate() {
                cmd_table.set(j + 1, arg.clone())?;
            }
            spawn_at_startup.set(i + 1, cmd_table)?;
        }
        config_table.set("spawn_at_startup", spawn_at_startup)?;

        // Spawn sh at startup commands
        let spawn_sh_at_startup = lua.create_table()?;
        for (i, spawn) in config.spawn_sh_at_startup.iter().enumerate() {
            spawn_sh_at_startup.set(i + 1, spawn.command.clone())?;
        }
        config_table.set("spawn_sh_at_startup", spawn_sh_at_startup)?;

        // Prefer no CSD
        config_table.set("prefer_no_csd", config.prefer_no_csd)?;

        // Screenshot path
        if let Some(path) = &config.screenshot_path.0 {
            config_table.set("screenshot_path", path.clone())?;
        }

        // Environment variables
        let environment = lua.create_table()?;
        for env_var in &config.environment.0 {
            if let Some(value) = &env_var.value {
                environment.set(env_var.name.clone(), value.clone())?;
            } else {
                environment.set(env_var.name.clone(), mlua::Value::Nil)?;
            }
        }
        config_table.set("environment", environment)?;

        Ok(())
    }

    /// Helper to convert PresetSize to Lua string representation
    fn preset_size_to_lua_string(size: &niri_config::PresetSize) -> String {
        match size {
            niri_config::PresetSize::Proportion(p) => format!("proportion {}", p),
            niri_config::PresetSize::Fixed(px) => format!("fixed {}", px),
        }
    }

    /// Helper to convert Color to hex string
    fn color_to_hex(color: &niri_config::Color) -> String {
        format!("#{:02x}{:02x}{:02x}{:02x}", 
            (color.r * 255.) as u8,
            (color.g * 255.) as u8,
            (color.b * 255.) as u8,
            (color.a * 255.) as u8)
    }

    /// Helper to convert Color to hex string without alpha
    fn color_to_hex_noalpha(color: &niri_config::Color) -> String {
        format!("#{:02x}{:02x}{:02x}", 
            (color.r * 255.) as u8,
            (color.g * 255.) as u8,
            (color.b * 255.) as u8)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::{create_config_lua_env, get_lua_global, assert_lua_table_has_key};

    /// Helper to setup config API and get the config table
    #[track_caller]
    fn setup_config_api() -> (Lua, LuaTable) {
        let lua = create_config_lua_env().unwrap();
        let niri_table: LuaTable = get_lua_global(&lua, "niri").unwrap();
        let config_table: LuaTable = niri_table.get("config").unwrap();
        (lua, config_table)
    }

    #[test]
    fn test_config_api_registration() {
        let (_lua, config_table) = setup_config_api();
        
        // Verify all subsystems are registered
        assert!(config_table.get::<LuaTable>("animations").is_ok());
        assert!(config_table.get::<LuaTable>("input").is_ok());
        assert!(config_table.get::<LuaTable>("layout").is_ok());
        assert!(config_table.get::<LuaTable>("cursor").is_ok());
        assert!(config_table.get::<LuaTable>("output").is_ok());
        assert!(config_table.get::<LuaTable>("gestures").is_ok());
        assert!(config_table.get::<LuaTable>("overview").is_ok());
        assert!(config_table.get::<LuaTable>("debug").is_ok());
        assert!(config_table.get::<LuaTable>("clipboard").is_ok());
        assert!(config_table.get::<LuaTable>("hotkey_overlay").is_ok());
        assert!(config_table.get::<LuaTable>("config_notification").is_ok());
        assert!(config_table.get::<LuaTable>("xwayland_satellite").is_ok());
    }

    #[test]
    fn test_animations_api() {
        let (_lua, config_table) = setup_config_api();
        let animations: LuaTable = config_table.get("animations").unwrap();

        // Verify animation settings are accessible
        assert_eq!(animations.get::<bool>("off").unwrap(), false);
        assert_eq!(animations.get::<f64>("slowdown").unwrap(), 1.0);

        // Verify animation tables exist
        assert!(animations.get::<LuaTable>("workspace_switch").is_ok());
        assert!(animations.get::<LuaTable>("window_open").is_ok());
        assert!(animations.get::<LuaTable>("window_close").is_ok());
    }

    #[test]
    fn test_animations_api_snapshot() {
        let (_lua, config_table) = setup_config_api();
        let animations: LuaTable = config_table.get("animations").unwrap();
        
        let off: bool = animations.get("off").unwrap();
        let slowdown: f64 = animations.get("slowdown").unwrap();
        
        insta::assert_debug_snapshot!("animations_global", (off, slowdown));
    }

    #[test]
    fn test_layout_api() {
        let (_lua, config_table) = setup_config_api();
        let layout: LuaTable = config_table.get("layout").unwrap();

        // Verify layout sections exist
        assert_lua_table_has_key(&layout, "struts");
        assert_lua_table_has_key(&layout, "focus_ring");
        assert_lua_table_has_key(&layout, "border");
        assert_lua_table_has_key(&layout, "shadow");
        assert_lua_table_has_key(&layout, "tab_indicator");
        assert_lua_table_has_key(&layout, "insert_hint");
    }

    #[test]
    fn test_input_api() {
        let (_lua, config_table) = setup_config_api();
        let input: LuaTable = config_table.get("input").unwrap();

        // Verify input device sections exist
        assert_lua_table_has_key(&input, "keyboard");
        assert_lua_table_has_key(&input, "mouse");
        assert_lua_table_has_key(&input, "touchpad");
        assert_lua_table_has_key(&input, "trackpoint");
    }

    #[test]
    fn test_overview_api() {
        let (_lua, config_table) = setup_config_api();
        let overview: LuaTable = config_table.get("overview").unwrap();

        // Verify overview settings are accessible
        assert_eq!(overview.get::<f64>("zoom").unwrap(), 0.5);
        assert_lua_table_has_key(&overview, "workspace_shadow");
    }

    #[test]
    fn test_gestures_api() {
        let (_lua, config_table) = setup_config_api();
        let gestures: LuaTable = config_table.get("gestures").unwrap();

        // Verify gesture sections exist
        assert_lua_table_has_key(&gestures, "dnd_edge_view_scroll");
        assert_lua_table_has_key(&gestures, "dnd_edge_workspace_switch");
        assert_lua_table_has_key(&gestures, "hot_corners");
    }

    #[test]
    fn test_cursor_api() {
        let (_lua, config_table) = setup_config_api();
        let cursor: LuaTable = config_table.get("cursor").unwrap();

        // Verify cursor settings are accessible
        assert_lua_table_has_key(&cursor, "xcursor_theme");
        assert_lua_table_has_key(&cursor, "xcursor_size");
        assert_lua_table_has_key(&cursor, "hide_when_typing");
    }

    #[test]
    fn test_debug_api() {
        let (_lua, config_table) = setup_config_api();
        let debug: LuaTable = config_table.get("debug").unwrap();

        // Verify debug settings are accessible
        assert_lua_table_has_key(&debug, "dbus_interfaces_in_non_session_instances");
        assert_lua_table_has_key(&debug, "wait_for_frame_completion_before_queueing");
    }

    #[test]
    fn test_clipboard_api() {
        let (_lua, config_table) = setup_config_api();
        let clipboard: LuaTable = config_table.get("clipboard").unwrap();

        // Verify clipboard settings are accessible
        assert_lua_table_has_key(&clipboard, "disable_primary");
    }

    #[test]
    fn test_hotkey_overlay_api() {
        let (_lua, config_table) = setup_config_api();
        let hotkey: LuaTable = config_table.get("hotkey_overlay").unwrap();

        // Verify hotkey overlay settings are accessible
        assert_lua_table_has_key(&hotkey, "skip_at_startup");
        assert_lua_table_has_key(&hotkey, "hide_not_bound");
    }

    #[test]
    fn test_config_notification_api() {
        let (_lua, config_table) = setup_config_api();
        let notif: LuaTable = config_table.get("config_notification").unwrap();

        // Verify config notification settings are accessible
        assert_lua_table_has_key(&notif, "disable_failed");
    }

    #[test]
    fn test_xwayland_satellite_api() {
        let (_lua, config_table) = setup_config_api();
        let xwayland: LuaTable = config_table.get("xwayland_satellite").unwrap();

        // Verify xwayland satellite settings are accessible
        assert_lua_table_has_key(&xwayland, "off");
        assert_lua_table_has_key(&xwayland, "path");
    }
}
