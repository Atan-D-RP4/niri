//! # Lua Table Extraction
//!
//! This module provides the `FromLuaTable` trait for type-safe extraction of
//! Rust configuration types from Lua tables.
//!
//! ## Example Usage
//!
//! ```rust,ignore
//! use mlua::{Lua, Table};
//! use niri_lua::extractors::FromLuaTable;
//! use niri_config::HotkeyOverlay;
//!
//! fn extract_config(lua: &Lua) -> mlua::Result<Option<HotkeyOverlay>> {
//!     let table: Table = lua.globals().get("config")?;
//!     HotkeyOverlay::from_lua_table(&table)
//! }
//! ```
//!
//! ## Implementing FromLuaTable
//!
//! ```rust,ignore
//! impl FromLuaTable for MyConfig {
//!     fn from_lua_table(table: &LuaTable) -> LuaResult<Option<Self>> {
//!         let field1 = extract_string_opt(table, "field1")?;
//!         let field2 = extract_bool_opt(table, "field2")?;
//!         
//!         // Return None if no relevant fields present
//!         if field1.is_none() && field2.is_none() {
//!             return Ok(None);
//!         }
//!         
//!         Ok(Some(MyConfig { field1, field2 }))
//!     }
//! }
//! ```

use std::str::FromStr;

use mlua::prelude::*;
use mlua::LuaSerdeExt;
use niri_config::appearance::*;
use niri_config::debug::{Debug, PreviewRender};
use niri_config::gestures::Gestures;
use niri_config::input::*;
use niri_config::layout::*;
use niri_config::misc::*;
use niri_config::recent_windows::{MruHighlight, MruPreviews, RecentWindows};
use niri_config::{ConfigNotification, FloatOrInt, XwaylandSatellite};
pub use niri_lua_traits::{
    extract_bool_opt, extract_float_opt, extract_int_opt, extract_string_opt, extract_table_opt,
    ExtractField, FromLuaTable,
};

pub fn extract_color(color_str: &str) -> Option<Color> {
    Color::from_str(color_str).ok()
}

pub fn extract_color_opt(table: &LuaTable, field: &str) -> LuaResult<Option<Color>> {
    if let Some(color_str) = extract_string_opt(table, field)? {
        Ok(extract_color(&color_str))
    } else {
        Ok(None)
    }
}

/// Extract clipboard configuration from Lua table.
pub fn extract_clipboard(table: &LuaTable) -> LuaResult<Option<Clipboard>> {
    if let Some(disable_primary) = extract_bool_opt(table, "disable_primary")? {
        Ok(Some(Clipboard { disable_primary }))
    } else {
        Ok(None)
    }
}

/// Extract overview configuration from Lua table.
pub fn extract_overview(table: &LuaTable) -> LuaResult<Option<Overview>> {
    let zoom = extract_float_opt(table, "zoom")?;
    let backdrop_color = extract_color_opt(table, "backdrop_color")?;

    if zoom.is_some() || backdrop_color.is_some() {
        let mut overview = Overview::default();
        if let Some(z) = zoom {
            overview.zoom = z;
        }
        if let Some(c) = backdrop_color {
            overview.backdrop_color = c;
        }
        Ok(Some(overview))
    } else {
        Ok(None)
    }
}

/// Extract environment variables from Lua table.
pub fn extract_environment(table: &LuaTable) -> LuaResult<Option<Environment>> {
    let mut vars = Vec::new();

    // Iterate through all key-value pairs in the table
    for pair in table.pairs::<String, LuaValue>() {
        let (key, value) = pair?;
        let value_str = match value {
            LuaValue::String(s) => Some(s.to_string_lossy().to_string()),
            LuaValue::Nil => None,
            _ => continue,
        };

        vars.push(EnvironmentVariable {
            name: key,
            value: value_str,
        });
    }

    if !vars.is_empty() {
        Ok(Some(Environment(vars)))
    } else {
        Ok(None)
    }
}

// ============================================================================
// Input configuration extractors (serde-based)
// ============================================================================

/// Extract XKB configuration from Lua value using serde.
pub fn extract_xkb(lua: &Lua, value: LuaValue) -> LuaResult<Option<Xkb>> {
    if value.is_nil() {
        return Ok(None);
    }
    lua.from_value(value).map(Some)
}

/// Extract keyboard configuration from Lua value using serde.
pub fn extract_keyboard(lua: &Lua, value: LuaValue) -> LuaResult<Option<Keyboard>> {
    if value.is_nil() {
        return Ok(None);
    }
    lua.from_value(value).map(Some)
}

/// Extract touchpad configuration from Lua value using serde.
pub fn extract_touchpad(lua: &Lua, value: LuaValue) -> LuaResult<Option<Touchpad>> {
    if value.is_nil() {
        return Ok(None);
    }
    lua.from_value(value).map(Some)
}

/// Extract mouse configuration from Lua value using serde.
pub fn extract_mouse(lua: &Lua, value: LuaValue) -> LuaResult<Option<Mouse>> {
    if value.is_nil() {
        return Ok(None);
    }
    lua.from_value(value).map(Some)
}

/// Extract trackpoint configuration from Lua value using serde.
pub fn extract_trackpoint(lua: &Lua, value: LuaValue) -> LuaResult<Option<Trackpoint>> {
    if value.is_nil() {
        return Ok(None);
    }
    lua.from_value(value).map(Some)
}

/// Extract touch configuration from Lua value using serde.
pub fn extract_touch(lua: &Lua, value: LuaValue) -> LuaResult<Option<Touch>> {
    if value.is_nil() {
        return Ok(None);
    }
    lua.from_value(value).map(Some)
}

/// Extract full Input configuration from Lua value using serde.
pub fn extract_input(lua: &Lua, value: LuaValue) -> LuaResult<Option<Input>> {
    if value.is_nil() {
        return Ok(None);
    }
    lua.from_value(value).map(Some)
}

// Legacy table-based extractors (for backward compatibility with existing call sites)
// These wrap the serde-based versions for code that still passes &LuaTable

/// Extract XKB configuration from Lua table (legacy wrapper).
pub fn extract_xkb_from_table(lua: &Lua, table: &LuaTable) -> LuaResult<Option<Xkb>> {
    extract_xkb(lua, LuaValue::Table(table.clone()))
}

/// Extract keyboard configuration from Lua table (legacy wrapper).
pub fn extract_keyboard_from_table(lua: &Lua, table: &LuaTable) -> LuaResult<Option<Keyboard>> {
    extract_keyboard(lua, LuaValue::Table(table.clone()))
}

/// Extract touchpad configuration from Lua table (legacy wrapper).
pub fn extract_touchpad_from_table(lua: &Lua, table: &LuaTable) -> LuaResult<Option<Touchpad>> {
    extract_touchpad(lua, LuaValue::Table(table.clone()))
}

/// Extract mouse configuration from Lua table (legacy wrapper).
pub fn extract_mouse_from_table(lua: &Lua, table: &LuaTable) -> LuaResult<Option<Mouse>> {
    extract_mouse(lua, LuaValue::Table(table.clone()))
}

/// Extract trackpoint configuration from Lua table (legacy wrapper).
pub fn extract_trackpoint_from_table(lua: &Lua, table: &LuaTable) -> LuaResult<Option<Trackpoint>> {
    extract_trackpoint(lua, LuaValue::Table(table.clone()))
}

/// Extract touch configuration from Lua table (legacy wrapper).
pub fn extract_touch_from_table(lua: &Lua, table: &LuaTable) -> LuaResult<Option<Touch>> {
    extract_touch(lua, LuaValue::Table(table.clone()))
}

/// Extract full Input configuration from Lua table (legacy wrapper).
pub fn extract_input_from_table(lua: &Lua, table: &LuaTable) -> LuaResult<Option<Input>> {
    extract_input(lua, LuaValue::Table(table.clone()))
}

// ============================================================================
// Output configuration extractors
// ============================================================================

// ============================================================================
// Layout configuration extractors
// ============================================================================

/// Extract layout configuration from Lua table.
pub fn extract_layout(table: &LuaTable) -> LuaResult<Option<Layout>> {
    let gaps = extract_float_opt(table, "gaps")?;
    let center_focused_column = extract_string_opt(table, "center_focused_column")?;
    let focus_ring = if let Some(fr_table) = extract_table_opt(table, "focus_ring")? {
        extract_focus_ring(&fr_table)?
    } else {
        None
    };
    let border = if let Some(b_table) = extract_table_opt(table, "border")? {
        extract_border(&b_table)?
    } else {
        None
    };
    let shadow = if let Some(s_table) = extract_table_opt(table, "shadow")? {
        extract_shadow(&s_table)?
    } else {
        None
    };
    let preset_column_widths = extract_preset_sizes(table, "preset_column_widths")?;
    let default_column_width =
        if let Some(dcw_table) = extract_table_opt(table, "default_column_width")? {
            extract_size_change(&dcw_table)?
        } else {
            None
        };
    let preset_window_heights = extract_preset_sizes(table, "preset_window_heights")?;

    if gaps.is_some()
        || center_focused_column.is_some()
        || focus_ring.is_some()
        || border.is_some()
        || shadow.is_some()
        || preset_column_widths.is_some()
        || default_column_width.is_some()
        || preset_window_heights.is_some()
    {
        let mut layout = Layout::default();
        if let Some(g) = gaps {
            layout.gaps = g;
        }
        if let Some(cfc) = center_focused_column {
            layout.center_focused_column = match cfc.as_str() {
                "never" => CenterFocusedColumn::Never,
                "always" => CenterFocusedColumn::Always,
                "on-overflow" => CenterFocusedColumn::OnOverflow,
                _ => CenterFocusedColumn::Never,
            };
        }
        if let Some(fr) = focus_ring {
            layout.focus_ring = fr;
        }
        if let Some(b) = border {
            layout.border = b;
        }
        if let Some(s) = shadow {
            layout.shadow = s;
        }
        if let Some(pcw) = preset_column_widths {
            layout.preset_column_widths = pcw;
        }
        if let Some(dcw) = default_column_width {
            layout.default_column_width = Some(dcw);
        }
        if let Some(pwh) = preset_window_heights {
            layout.preset_window_heights = pwh;
        }
        Ok(Some(layout))
    } else {
        Ok(None)
    }
}

/// Extract focus_ring configuration from Lua table.
pub fn extract_focus_ring(table: &LuaTable) -> LuaResult<Option<FocusRing>> {
    let off = extract_bool_opt(table, "off")?;
    let width = extract_float_opt(table, "width")?;
    let active_color = extract_color_opt(table, "active_color")?;
    let inactive_color = extract_color_opt(table, "inactive_color")?;

    if off.is_some() || width.is_some() || active_color.is_some() || inactive_color.is_some() {
        let mut focus_ring = FocusRing::default();
        if let Some(v) = off {
            focus_ring.off = v;
        }
        if let Some(v) = width {
            focus_ring.width = v;
        }
        if let Some(c) = active_color {
            focus_ring.active_color = c;
        }
        if let Some(c) = inactive_color {
            focus_ring.inactive_color = c;
        }
        Ok(Some(focus_ring))
    } else {
        Ok(None)
    }
}

/// Extract border configuration from Lua table.
pub fn extract_border(table: &LuaTable) -> LuaResult<Option<Border>> {
    let off = extract_bool_opt(table, "off")?;
    let width = extract_float_opt(table, "width")?;
    let active_color = extract_color_opt(table, "active_color")?;
    let inactive_color = extract_color_opt(table, "inactive_color")?;
    let urgent_color = extract_color_opt(table, "urgent_color")?;

    if off.is_some()
        || width.is_some()
        || active_color.is_some()
        || inactive_color.is_some()
        || urgent_color.is_some()
    {
        let mut border = Border::default();
        if let Some(v) = off {
            border.off = v;
        }
        if let Some(v) = width {
            border.width = v;
        }
        if let Some(c) = active_color {
            border.active_color = c;
        }
        if let Some(c) = inactive_color {
            border.inactive_color = c;
        }
        if let Some(c) = urgent_color {
            border.urgent_color = c;
        }
        Ok(Some(border))
    } else {
        Ok(None)
    }
}

/// Extract shadow configuration from Lua table.
pub fn extract_shadow(table: &LuaTable) -> LuaResult<Option<Shadow>> {
    let off = extract_bool_opt(table, "off")?;
    let on = extract_bool_opt(table, "on")?;
    let softness = extract_float_opt(table, "softness")?;
    let spread = extract_float_opt(table, "spread")?;
    let color = extract_color_opt(table, "color")?;
    let inactive_color = extract_color_opt(table, "inactive_color")?;
    let draw_behind_window = extract_bool_opt(table, "draw_behind_window")?;
    let offset = if let Some(offset_table) = extract_table_opt(table, "offset")? {
        let x = extract_float_opt(&offset_table, "x")?.unwrap_or(0.0);
        let y = extract_float_opt(&offset_table, "y")?.unwrap_or(0.0);
        Some((x, y))
    } else {
        None
    };

    if off.is_some()
        || on.is_some()
        || softness.is_some()
        || spread.is_some()
        || color.is_some()
        || offset.is_some()
        || inactive_color.is_some()
        || draw_behind_window.is_some()
    {
        let mut shadow = Shadow::default();
        // Handle both "off" (inverted) and "on" directly
        if let Some(v) = off {
            shadow.on = !v;
        }
        if let Some(v) = on {
            shadow.on = v;
        }
        if let Some(v) = softness {
            shadow.softness = v;
        }
        if let Some(v) = spread {
            shadow.spread = v;
        }
        if let Some(c) = color {
            shadow.color = c;
        }
        if let Some(c) = inactive_color {
            shadow.inactive_color = Some(c);
        }
        if let Some(v) = draw_behind_window {
            shadow.draw_behind_window = v;
        }
        if let Some((x, y)) = offset {
            shadow.offset = ShadowOffset {
                x: FloatOrInt(x),
                y: FloatOrInt(y),
            };
        }
        Ok(Some(shadow))
    } else {
        Ok(None)
    }
}

/// Extract preset sizes (column widths or window heights) from Lua table.
fn extract_preset_sizes(table: &LuaTable, field: &str) -> LuaResult<Option<Vec<PresetSize>>> {
    if let Some(array_table) = extract_table_opt(table, field)? {
        let mut sizes = Vec::new();
        // Iterate as array (1-indexed in Lua)
        for i in 1..=array_table.len()? {
            if let Ok(item_table) = array_table.get::<LuaTable>(i) {
                if let Some(size) = extract_size_change(&item_table)? {
                    sizes.push(size);
                }
            }
        }
        if !sizes.is_empty() {
            return Ok(Some(sizes));
        }
    }
    Ok(None)
}

// ============================================================================
// Window rule extractors
// ============================================================================

/// Extract a single size change (proportion or fixed).
fn extract_size_change(table: &LuaTable) -> LuaResult<Option<PresetSize>> {
    if let Some(proportion) = extract_float_opt(table, "proportion")? {
        return Ok(Some(PresetSize::Proportion(proportion)));
    }
    if let Some(fixed) = extract_int_opt(table, "fixed")? {
        return Ok(Some(PresetSize::Fixed(fixed as i32)));
    }
    Ok(None)
}

// ============================================================================
// Gestures configuration extractors
// ============================================================================

/// Extract gestures configuration from Lua table.
pub fn extract_gestures(table: &LuaTable) -> LuaResult<Option<Gestures>> {
    // Gestures currently has limited config options (just hot corners).
    // For now, return a default if the table has any values
    if table.len()? == 0 {
        Ok(None)
    } else {
        Ok(Some(Gestures::default()))
    }
}

// ============================================================================
// Recent Windows (MRU) configuration extractors
// ============================================================================

/// Extract recent windows configuration from Lua table.
pub fn extract_recent_windows(table: &LuaTable) -> LuaResult<Option<RecentWindows>> {
    let off = extract_bool_opt(table, "off")?;
    let on = extract_bool_opt(table, "on")?;
    let open_delay_ms = extract_int_opt(table, "open_delay_ms")?.map(|v| v as u16);

    let highlight = if let Some(hl_table) = extract_table_opt(table, "highlight")? {
        extract_mru_highlight(&hl_table)?
    } else {
        None
    };

    let previews = if let Some(pv_table) = extract_table_opt(table, "previews")? {
        extract_mru_previews(&pv_table)?
    } else {
        None
    };

    if off.is_some()
        || on.is_some()
        || open_delay_ms.is_some()
        || highlight.is_some()
        || previews.is_some()
    {
        let mut recent_windows = RecentWindows::default();
        // Handle "off" (inverted) or "on" directly
        if let Some(v) = off {
            recent_windows.on = !v;
        }
        if let Some(v) = on {
            recent_windows.on = v;
        }
        if let Some(d) = open_delay_ms {
            recent_windows.open_delay_ms = d;
        }
        if let Some(hl) = highlight {
            recent_windows.highlight = hl;
        }
        if let Some(pv) = previews {
            recent_windows.previews = pv;
        }
        Ok(Some(recent_windows))
    } else {
        Ok(None)
    }
}

/// Extract MRU highlight configuration from Lua table.
fn extract_mru_highlight(table: &LuaTable) -> LuaResult<Option<MruHighlight>> {
    let active_color = extract_color_opt(table, "active_color")?;
    let urgent_color = extract_color_opt(table, "urgent_color")?;
    let padding = extract_float_opt(table, "padding")?;
    let corner_radius = extract_float_opt(table, "corner_radius")?;

    if active_color.is_some()
        || urgent_color.is_some()
        || padding.is_some()
        || corner_radius.is_some()
    {
        let mut highlight = MruHighlight::default();
        if let Some(c) = active_color {
            highlight.active_color = c;
        }
        if let Some(c) = urgent_color {
            highlight.urgent_color = c;
        }
        if let Some(p) = padding {
            highlight.padding = p;
        }
        if let Some(r) = corner_radius {
            highlight.corner_radius = r;
        }
        Ok(Some(highlight))
    } else {
        Ok(None)
    }
}

/// Extract MRU previews configuration from Lua table.
fn extract_mru_previews(table: &LuaTable) -> LuaResult<Option<MruPreviews>> {
    let max_height = extract_float_opt(table, "max_height")?;
    let max_scale = extract_float_opt(table, "max_scale")?;

    if max_height.is_some() || max_scale.is_some() {
        let mut previews = MruPreviews::default();
        if let Some(h) = max_height {
            previews.max_height = h;
        }
        if let Some(s) = max_scale {
            previews.max_scale = s;
        }
        Ok(Some(previews))
    } else {
        Ok(None)
    }
}

// ============================================================================
// Config notification extractors
// ============================================================================

/// Extract config notification settings from Lua table.
pub fn extract_config_notification(table: &LuaTable) -> LuaResult<Option<ConfigNotification>> {
    let disable_failed = extract_bool_opt(table, "disable_failed")?;

    if disable_failed.is_some() {
        let mut notification = ConfigNotification::default();
        if let Some(v) = disable_failed {
            notification.disable_failed = v;
        }
        Ok(Some(notification))
    } else {
        Ok(None)
    }
}

// ============================================================================
// Debug configuration extractors
// ============================================================================

/// Extract debug configuration from Lua table.
pub fn extract_debug(table: &LuaTable) -> LuaResult<Option<Debug>> {
    let preview_render = extract_string_opt(table, "preview_render")?;
    let dbus_interfaces_in_non_session_instances =
        extract_bool_opt(table, "dbus_interfaces_in_non_session_instances")?;
    let wait_for_frame_completion_before_queueing =
        extract_bool_opt(table, "wait_for_frame_completion_before_queueing")?;
    let enable_overlay_planes = extract_bool_opt(table, "enable_overlay_planes")?;
    let disable_cursor_plane = extract_bool_opt(table, "disable_cursor_plane")?;
    let disable_direct_scanout = extract_bool_opt(table, "disable_direct_scanout")?;
    let keep_max_bpc_unchanged = extract_bool_opt(table, "keep_max_bpc_unchanged")?;
    let restrict_primary_scanout_to_matching_format =
        extract_bool_opt(table, "restrict_primary_scanout_to_matching_format")?;
    let render_drm_device = extract_string_opt(table, "render_drm_device")?;
    let ignored_drm_devices = extract_table_opt(table, "ignored_drm_devices")?;
    let force_pipewire_invalid_modifier =
        extract_bool_opt(table, "force_pipewire_invalid_modifier")?;
    let emulate_zero_presentation_time = extract_bool_opt(table, "emulate_zero_presentation_time")?;
    let disable_resize_throttling = extract_bool_opt(table, "disable_resize_throttling")?;
    let disable_transactions = extract_bool_opt(table, "disable_transactions")?;
    let keep_laptop_panel_on_when_lid_is_closed =
        extract_bool_opt(table, "keep_laptop_panel_on_when_lid_is_closed")?;
    let disable_monitor_names = extract_bool_opt(table, "disable_monitor_names")?;
    let strict_new_window_focus_policy = extract_bool_opt(table, "strict_new_window_focus_policy")?;
    let honor_xdg_activation_with_invalid_serial =
        extract_bool_opt(table, "honor_xdg_activation_with_invalid_serial")?;
    let deactivate_unfocused_windows = extract_bool_opt(table, "deactivate_unfocused_windows")?;
    let skip_cursor_only_updates_during_vrr =
        extract_bool_opt(table, "skip_cursor_only_updates_during_vrr")?;

    if preview_render.is_some()
        || dbus_interfaces_in_non_session_instances.is_some()
        || wait_for_frame_completion_before_queueing.is_some()
        || enable_overlay_planes.is_some()
        || disable_cursor_plane.is_some()
        || disable_direct_scanout.is_some()
        || keep_max_bpc_unchanged.is_some()
        || restrict_primary_scanout_to_matching_format.is_some()
        || render_drm_device.is_some()
        || ignored_drm_devices.is_some()
        || force_pipewire_invalid_modifier.is_some()
        || emulate_zero_presentation_time.is_some()
        || disable_resize_throttling.is_some()
        || disable_transactions.is_some()
        || keep_laptop_panel_on_when_lid_is_closed.is_some()
        || disable_monitor_names.is_some()
        || strict_new_window_focus_policy.is_some()
        || honor_xdg_activation_with_invalid_serial.is_some()
        || deactivate_unfocused_windows.is_some()
        || skip_cursor_only_updates_during_vrr.is_some()
    {
        let mut debug = Debug::default();

        if let Some(value) = preview_render {
            debug.preview_render = match value.as_str() {
                "screencast" => Some(PreviewRender::Screencast),
                "screen_capture" => Some(PreviewRender::ScreenCapture),
                _ => None,
            };
        }
        if let Some(v) = dbus_interfaces_in_non_session_instances {
            debug.dbus_interfaces_in_non_session_instances = v;
        }
        if let Some(v) = wait_for_frame_completion_before_queueing {
            debug.wait_for_frame_completion_before_queueing = v;
        }
        if let Some(v) = enable_overlay_planes {
            debug.enable_overlay_planes = v;
        }
        if let Some(v) = disable_cursor_plane {
            debug.disable_cursor_plane = v;
        }
        if let Some(v) = disable_direct_scanout {
            debug.disable_direct_scanout = v;
        }
        if let Some(v) = keep_max_bpc_unchanged {
            debug.keep_max_bpc_unchanged = v;
        }
        if let Some(v) = restrict_primary_scanout_to_matching_format {
            debug.restrict_primary_scanout_to_matching_format = v;
        }
        if let Some(v) = render_drm_device {
            debug.render_drm_device = Some(v.into());
        }
        if let Some(devices) = ignored_drm_devices {
            for pair in devices.sequence_values::<LuaValue>() {
                let value = pair?;
                if let LuaValue::String(s) = value {
                    debug.ignored_drm_devices.push(s.to_string_lossy().into());
                }
            }
        }
        if let Some(v) = force_pipewire_invalid_modifier {
            debug.force_pipewire_invalid_modifier = v;
        }
        if let Some(v) = emulate_zero_presentation_time {
            debug.emulate_zero_presentation_time = v;
        }
        if let Some(v) = disable_resize_throttling {
            debug.disable_resize_throttling = v;
        }
        if let Some(v) = disable_transactions {
            debug.disable_transactions = v;
        }
        if let Some(v) = keep_laptop_panel_on_when_lid_is_closed {
            debug.keep_laptop_panel_on_when_lid_is_closed = v;
        }
        if let Some(v) = disable_monitor_names {
            debug.disable_monitor_names = v;
        }
        if let Some(v) = strict_new_window_focus_policy {
            debug.strict_new_window_focus_policy = v;
        }
        if let Some(v) = honor_xdg_activation_with_invalid_serial {
            debug.honor_xdg_activation_with_invalid_serial = v;
        }
        if let Some(v) = deactivate_unfocused_windows {
            debug.deactivate_unfocused_windows = v;
        }
        if let Some(v) = skip_cursor_only_updates_during_vrr {
            debug.skip_cursor_only_updates_during_vrr = v;
        }

        Ok(Some(debug))
    } else {
        Ok(None)
    }
}

// ============================================================================
// Xwayland Satellite extractors
// ============================================================================

/// Extract xwayland_satellite configuration from Lua table.
pub fn extract_xwayland_satellite(table: &LuaTable) -> LuaResult<Option<XwaylandSatellite>> {
    let off = extract_bool_opt(table, "off")?;

    if off.is_some() {
        let mut xwayland = XwaylandSatellite::default();
        if let Some(v) = off {
            xwayland.off = v;
        }
        Ok(Some(xwayland))
    } else {
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::create_test_lua_table;

    #[test]
    fn screenshot_path_from_lua_table_supports_none_and_false() {
        let (_lua, table) = create_test_table();
        table.set("path", false).unwrap();

        let result = ScreenshotPath::from_lua_table(&table).unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap().0, None);
    }

    #[test]
    fn debug_from_lua_table_extracts_multiple_fields() {
        let (_lua, table) = create_test_table();
        table.set("disable_direct_scanout", true).unwrap();
        table
            .set("render_drm_device", "/dev/dri/renderD128")
            .unwrap();
        table
            .set(
                "ignored_drm_devices",
                vec!["/dev/dri/card0", "/dev/dri/card1"],
            )
            .unwrap();

        let result = extract_debug(&table).unwrap();
        assert!(result.is_some());
        let debug = result.unwrap();
        assert!(debug.disable_direct_scanout);
        assert_eq!(
            debug.render_drm_device.unwrap().to_string_lossy(),
            "/dev/dri/renderD128"
        );
        assert_eq!(debug.ignored_drm_devices.len(), 2);
    }

    // Re-export for backward compatibility with existing tests
    fn create_test_table() -> (mlua::Lua, mlua::Table) {
        create_test_lua_table()
    }

    // ========================================================================
    // extract_string_opt tests
    // ========================================================================

    #[test]
    fn extract_string_opt_with_value() {
        let (_lua, table) = create_test_table();
        table.set("key", "hello").unwrap();
        let result = extract_string_opt(&table, "key").unwrap();
        assert_eq!(result, Some("hello".to_string()));
    }

    #[test]
    fn extract_string_opt_with_nil() {
        let (_lua, table) = create_test_table();
        table.set("key", mlua::Nil).unwrap();
        let result = extract_string_opt(&table, "key").unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn extract_string_opt_missing_key() {
        let (_lua, table) = create_test_table();
        let result = extract_string_opt(&table, "nonexistent").unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn extract_string_opt_wrong_type_number() {
        let (_lua, table) = create_test_table();
        table.set("key", 42).unwrap();
        let result = extract_string_opt(&table, "key").unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn extract_string_opt_wrong_type_bool() {
        let (_lua, table) = create_test_table();
        table.set("key", true).unwrap();
        let result = extract_string_opt(&table, "key").unwrap();
        assert_eq!(result, None);
    }

    // ========================================================================
    // extract_bool_opt tests
    // ========================================================================

    #[test]
    fn extract_bool_opt_true() {
        let (_lua, table) = create_test_table();
        table.set("key", true).unwrap();
        let result = extract_bool_opt(&table, "key").unwrap();
        assert_eq!(result, Some(true));
    }

    #[test]
    fn extract_bool_opt_false() {
        let (_lua, table) = create_test_table();
        table.set("key", false).unwrap();
        let result = extract_bool_opt(&table, "key").unwrap();
        assert_eq!(result, Some(false));
    }

    #[test]
    fn extract_bool_opt_nil() {
        let (_lua, table) = create_test_table();
        table.set("key", mlua::Nil).unwrap();
        let result = extract_bool_opt(&table, "key").unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn extract_bool_opt_wrong_type_string() {
        let (_lua, table) = create_test_table();
        table.set("key", "true").unwrap();
        let result = extract_bool_opt(&table, "key").unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn extract_bool_opt_wrong_type_number() {
        let (_lua, table) = create_test_table();
        table.set("key", 1).unwrap();
        let result = extract_bool_opt(&table, "key").unwrap();
        assert_eq!(result, None);
    }

    // ========================================================================
    // extract_int_opt tests
    // ========================================================================

    #[test]
    fn extract_int_opt_positive() {
        let (_lua, table) = create_test_table();
        table.set("key", 42i64).unwrap();
        let result = extract_int_opt(&table, "key").unwrap();
        assert_eq!(result, Some(42));
    }

    #[test]
    fn extract_int_opt_negative() {
        let (_lua, table) = create_test_table();
        table.set("key", -100i64).unwrap();
        let result = extract_int_opt(&table, "key").unwrap();
        assert_eq!(result, Some(-100));
    }

    #[test]
    fn extract_int_opt_zero() {
        let (_lua, table) = create_test_table();
        table.set("key", 0i64).unwrap();
        let result = extract_int_opt(&table, "key").unwrap();
        assert_eq!(result, Some(0));
    }

    #[test]
    fn extract_int_opt_from_number() {
        let (_lua, table) = create_test_table();
        table.set("key", 42.0).unwrap();
        let result = extract_int_opt(&table, "key").unwrap();
        assert_eq!(result, Some(42));
    }

    #[test]
    fn extract_int_opt_from_float_truncate() {
        let (_lua, table) = create_test_table();
        table.set("key", 42.9).unwrap();
        let result = extract_int_opt(&table, "key").unwrap();
        assert_eq!(result, Some(42)); // Truncates to 42
    }

    #[test]
    fn extract_int_opt_nil() {
        let (_lua, table) = create_test_table();
        table.set("key", mlua::Nil).unwrap();
        let result = extract_int_opt(&table, "key").unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn extract_int_opt_wrong_type_string() {
        let (_lua, table) = create_test_table();
        table.set("key", "42").unwrap();
        let result = extract_int_opt(&table, "key").unwrap();
        assert_eq!(result, None);
    }

    // ========================================================================
    // extract_float_opt tests
    // ========================================================================

    #[test]
    #[allow(clippy::approx_constant)]
    fn extract_float_opt_decimal() {
        let (_lua, table) = create_test_table();
        table.set("key", 3.14).unwrap();
        let result = extract_float_opt(&table, "key").unwrap();
        assert_eq!(result, Some(3.14));
    }

    #[test]
    fn extract_float_opt_negative() {
        let (_lua, table) = create_test_table();
        table.set("key", -2.5).unwrap();
        let result = extract_float_opt(&table, "key").unwrap();
        assert_eq!(result, Some(-2.5));
    }

    #[test]
    fn extract_float_opt_from_integer() {
        let (_lua, table) = create_test_table();
        table.set("key", 42i64).unwrap();
        let result = extract_float_opt(&table, "key").unwrap();
        assert_eq!(result, Some(42.0));
    }

    #[test]
    fn extract_float_opt_nil() {
        let (_lua, table) = create_test_table();
        table.set("key", mlua::Nil).unwrap();
        let result = extract_float_opt(&table, "key").unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn extract_float_opt_wrong_type() {
        let (_lua, table) = create_test_table();
        table.set("key", "3.14").unwrap();
        let result = extract_float_opt(&table, "key").unwrap();
        assert_eq!(result, None);
    }

    // ========================================================================
    // extract_table_opt tests
    // ========================================================================

    #[test]
    fn extract_table_opt_with_table() {
        let (lua, table) = create_test_table();
        let inner = lua.create_table().unwrap();
        inner.set("inner_key", "inner_value").unwrap();
        table.set("key", inner).unwrap();

        let result = extract_table_opt(&table, "key").unwrap();
        assert!(result.is_some());
        let inner_table = result.unwrap();
        let value: String = inner_table.get("inner_key").unwrap();
        assert_eq!(value, "inner_value");
    }

    #[test]
    fn extract_table_opt_empty_table() {
        let (lua, table) = create_test_table();
        let inner = lua.create_table().unwrap();
        table.set("key", inner).unwrap();

        let result = extract_table_opt(&table, "key").unwrap();
        assert!(result.is_some());
        let inner_table = result.unwrap();
        let len: usize = inner_table.len().unwrap() as usize;
        assert_eq!(len, 0);
    }

    #[test]
    fn extract_table_opt_nil() {
        let (_lua, table) = create_test_table();
        table.set("key", mlua::Nil).unwrap();
        let result = extract_table_opt(&table, "key").unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn extract_table_opt_wrong_type_string() {
        let (_lua, table) = create_test_table();
        table.set("key", "not a table").unwrap();
        let result = extract_table_opt(&table, "key").unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn extract_table_opt_wrong_type_number() {
        let (_lua, table) = create_test_table();
        table.set("key", 42).unwrap();
        let result = extract_table_opt(&table, "key").unwrap();
        assert_eq!(result, None);
    }

    // ========================================================================
    // extract_color tests
    // ========================================================================

    #[test]
    fn extract_color_6digit_hex() {
        let color = extract_color("#FF0000");
        assert!(color.is_some());
    }

    #[test]
    fn extract_color_8digit_hex() {
        let color = extract_color("#FF0000FF");
        assert!(color.is_some());
    }

    #[test]
    fn extract_color_lowercase() {
        let color = extract_color("#ff0000");
        assert!(color.is_some());
    }

    #[test]
    fn extract_color_mixed_case() {
        let color = extract_color("#Ff00Ff");
        assert!(color.is_some());
    }

    #[test]
    fn extract_color_invalid_hex_chars() {
        let color = extract_color("#GGGGGG");
        assert!(color.is_none());
    }

    #[test]
    fn extract_color_missing_hash() {
        let color = extract_color("FF0000");
        // CSS color parser accepts colors without hash
        assert!(color.is_some());
    }

    #[test]
    fn extract_color_short_hex() {
        let color = extract_color("#F00");
        // CSS color parser accepts short hex formats
        assert!(color.is_some());
    }

    // ========================================================================
    // extract_color_opt tests
    // ========================================================================

    #[test]
    fn extract_color_opt_valid() {
        let (_lua, table) = create_test_table();
        table.set("color", "#FF0000").unwrap();
        let result = extract_color_opt(&table, "color").unwrap();
        assert!(result.is_some());
    }

    #[test]
    fn extract_color_opt_invalid() {
        let (_lua, table) = create_test_table();
        table.set("color", "#GGGGGG").unwrap();
        let result = extract_color_opt(&table, "color").unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn extract_color_opt_missing() {
        let (_lua, table) = create_test_table();
        let result = extract_color_opt(&table, "color").unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn extract_color_opt_nil() {
        let (_lua, table) = create_test_table();
        table.set("color", mlua::Nil).unwrap();
        let result = extract_color_opt(&table, "color").unwrap();
        assert_eq!(result, None);
    }

    // ========================================================================
    // extract_screenshot_path tests
    // ========================================================================

    // ========================================================================
    // output extractor tests
    // ========================================================================

    // ========================================================================
    // extract_animations tests
    // ========================================================================

    // ========================================================================
    // extract_clipboard tests
    // ========================================================================

    #[test]
    fn extract_clipboard_with_true() {
        let (_lua, table) = create_test_table();
        table.set("disable_primary", true).unwrap();
        let result = extract_clipboard(&table).unwrap();
        assert!(result.is_some());
        assert!(result.unwrap().disable_primary);
    }

    #[test]
    fn extract_clipboard_with_false() {
        let (_lua, table) = create_test_table();
        table.set("disable_primary", false).unwrap();
        let result = extract_clipboard(&table).unwrap();
        assert!(result.is_some());
        assert!(!result.unwrap().disable_primary);
    }

    #[test]
    fn extract_clipboard_empty() {
        let (_lua, table) = create_test_table();
        let result = extract_clipboard(&table).unwrap();
        assert_eq!(result, None);
    }

    // ========================================================================
    // extract_overview tests
    // ========================================================================

    #[test]
    fn extract_overview_with_zoom() {
        let (_lua, table) = create_test_table();
        table.set("zoom", 0.5).unwrap();
        let result = extract_overview(&table).unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap().zoom, 0.5);
    }

    #[test]
    fn extract_overview_with_backdrop_color() {
        let (_lua, table) = create_test_table();
        table.set("backdrop_color", "#000000").unwrap();
        let result = extract_overview(&table).unwrap();
        assert!(result.is_some());
    }

    #[test]
    fn extract_overview_with_both() {
        let (_lua, table) = create_test_table();
        table.set("zoom", 0.7).unwrap();
        table.set("backdrop_color", "#111111").unwrap();
        let result = extract_overview(&table).unwrap();
        assert!(result.is_some());
        let overview = result.unwrap();
        assert_eq!(overview.zoom, 0.7);
    }

    #[test]
    fn extract_overview_empty() {
        let (_lua, table) = create_test_table();
        let result = extract_overview(&table).unwrap();
        assert_eq!(result, None);
    }

    // ========================================================================
    // extract_environment tests
    // ========================================================================

    #[test]
    fn extract_environment_with_variables() {
        let (_lua, table) = create_test_table();
        table.set("VAR1", "value1").unwrap();
        table.set("VAR2", "value2").unwrap();

        let result = extract_environment(&table).unwrap();
        assert!(result.is_some());
        let env = result.unwrap();
        assert_eq!(env.0.len(), 2);
    }

    #[test]
    fn extract_environment_with_nil_value() {
        let (_lua, table) = create_test_table();
        table.set("VAR1", "value1").unwrap();
        table.set("VAR2", mlua::Nil).unwrap();

        let result = extract_environment(&table).unwrap();
        assert!(result.is_some());
        let env = result.unwrap();
        // Lua table.pairs() doesn't iterate over nil values, so only VAR1 is present
        assert_eq!(env.0.len(), 1);

        // Find the string variable
        let string_var = env.0.iter().find(|v| v.name == "VAR1").unwrap();
        assert_eq!(string_var.value, Some("value1".to_string()));
    }

    #[test]
    fn extract_environment_ignores_non_string_values() {
        let (_lua, table) = create_test_table();
        table.set("VAR1", "value1").unwrap();
        table.set("VAR2", 42).unwrap();

        let result = extract_environment(&table).unwrap();
        assert!(result.is_some());
        let env = result.unwrap();
        // Should only have VAR1 since VAR2 is not string-like
        assert_eq!(env.0.len(), 1);
        assert_eq!(env.0[0].name, "VAR1");
    }

    #[test]
    fn extract_environment_empty() {
        let (_lua, table) = create_test_table();
        let result = extract_environment(&table).unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn extract_environment_all_non_string_values() {
        let (_lua, table) = create_test_table();
        table.set("VAR1", 42).unwrap();
        table.set("VAR2", true).unwrap();

        let result = extract_environment(&table).unwrap();
        assert_eq!(result, None); // All ignored, so result is None
    }

    // ========================================================================
    // SNAPSHOT TESTS - Error Message Formats
    // ========================================================================

    #[test]
    fn snapshot_extract_string_opt_wrong_type_error() {
        let (_lua, table) = create_test_table();
        table.set("key", 42).unwrap();
        let result = extract_string_opt(&table, "key").unwrap();
        // Returns None for wrong type - this is graceful degradation, not an error
        insta::assert_debug_snapshot!("extract_string_opt_wrong_type", result);
    }

    #[test]
    fn snapshot_extract_bool_opt_wrong_type() {
        let (_lua, table) = create_test_table();
        table.set("key", "true").unwrap();
        let result = extract_bool_opt(&table, "key").unwrap();
        insta::assert_debug_snapshot!("extract_bool_opt_wrong_type", result);
    }

    #[test]
    fn snapshot_extract_int_opt_from_float_conversion() {
        let (_lua, table) = create_test_table();
        table.set("key", 42.9).unwrap();
        let result = extract_int_opt(&table, "key").unwrap();
        // Integer extraction truncates float
        insta::assert_debug_snapshot!("extract_int_opt_from_float", result);
    }

    #[test]
    fn snapshot_extract_color_valid_formats() {
        let color1 = extract_color("#FF0000");
        let color2 = extract_color("#FF0000FF");
        let color3 = extract_color("#ff0000");
        let color4 = extract_color("#F00");

        insta::assert_debug_snapshot!(
            "extract_color_valid",
            (
                color1.is_some(),
                color2.is_some(),
                color3.is_some(),
                color4.is_some(),
            )
        );
    }

    #[test]
    fn snapshot_extract_color_invalid_formats() {
        let color1 = extract_color("#GGGGGG");
        let color2 = extract_color("invalid");
        let color3 = extract_color("");

        insta::assert_debug_snapshot!(
            "extract_color_invalid",
            (color1.is_none(), color2.is_none(), color3.is_none(),)
        );
    }

    #[test]
    fn snapshot_extract_xkb_complete_config() {
        let (lua, table) = create_test_table();
        table.set("layout", "us,de").unwrap();
        table.set("model", "pc104").unwrap();
        table.set("variant", "dvorak").unwrap();
        table.set("options", "grp:alt_shift_toggle").unwrap();

        let result = extract_xkb_from_table(&lua, &table).unwrap();
        assert!(result.is_some());
        let xkb = result.unwrap();

        insta::assert_debug_snapshot!(
            "extract_xkb_complete",
            (
                xkb.layout.clone(),
                xkb.model.clone(),
                xkb.variant.clone(),
                xkb.options.clone(),
            )
        );
    }

    #[test]
    fn snapshot_extract_touchpad_config_structure() {
        let (lua, table) = create_test_table();
        table.set("tap", true).unwrap();
        table.set("natural_scroll", true).unwrap();
        table.set("accel_speed", 0.5).unwrap();
        table.set("accel_profile", "adaptive").unwrap();

        let result = extract_touchpad_from_table(&lua, &table).unwrap();
        assert!(result.is_some());
        let touchpad = result.unwrap();

        insta::assert_debug_snapshot!(
            "extract_touchpad_config",
            (
                touchpad.tap,
                touchpad.natural_scroll,
                touchpad.accel_speed.0,
            )
        );
    }

    #[test]
    fn snapshot_extract_focus_ring_config() {
        let (_lua, table) = create_test_table();
        table.set("off", false).unwrap();
        table.set("width", 4.0).unwrap();
        table.set("active_color", "#FF0000").unwrap();
        table.set("inactive_color", "#888888").unwrap();

        let result = extract_focus_ring(&table).unwrap();
        assert!(result.is_some());
        let focus_ring = result.unwrap();

        insta::assert_debug_snapshot!(
            "extract_focus_ring_config",
            (focus_ring.off, focus_ring.width,)
        );
    }

    #[test]
    fn snapshot_extract_environment_variables() {
        let (_lua, table) = create_test_table();
        table.set("PATH", "/usr/bin:/bin").unwrap();
        table.set("HOME", "/home/user").unwrap();
        table.set("WAYLAND_DISPLAY", "wayland-0").unwrap();

        let result = extract_environment(&table).unwrap();
        assert!(result.is_some());
        let env = result.unwrap();

        // Sort for consistent snapshot
        let mut names: Vec<_> = env.0.iter().map(|v| v.name.clone()).collect();
        names.sort();

        insta::assert_debug_snapshot!("extract_environment_var_names", names);
    }

    // ========================================================================
    // Additional Snapshot Tests for Error Messages and Edge Cases
    // ========================================================================

    #[test]
    fn snapshot_extractors_color_parsing_errors() {
        // Test various invalid color formats
        let invalid_hex = extract_color("#GGGGGG");
        let too_short = extract_color("#F");
        let too_long = extract_color("#FF00FF00AA");
        let no_hash_invalid = extract_color("GGGGGG");
        let empty = extract_color("");
        let rgb_format = extract_color("rgb(255,0,0)");

        insta::assert_debug_snapshot!(
            "extractors_color_parsing_errors",
            (
                invalid_hex,
                too_short,
                too_long,
                no_hash_invalid,
                empty,
                rgb_format,
            )
        );
    }

    #[test]
    fn snapshot_extractors_size_parsing_valid() {
        let (lua, _) = create_test_table();

        // Proportion size
        let proportion_table = lua.create_table().unwrap();
        proportion_table.set("proportion", 0.5).unwrap();
        let proportion_result = extract_size_change(&proportion_table).unwrap();

        // Fixed size
        let fixed_table = lua.create_table().unwrap();
        fixed_table.set("fixed", 1920i64).unwrap();
        let fixed_result = extract_size_change(&fixed_table).unwrap();

        insta::assert_debug_snapshot!(
            "extractors_size_parsing_valid",
            (proportion_result, fixed_result,)
        );
    }

    #[test]
    fn snapshot_extractors_size_parsing_edge_cases() {
        let (lua, _) = create_test_table();

        // Empty table (no proportion or fixed)
        let empty_table = lua.create_table().unwrap();
        let empty_result = extract_size_change(&empty_table).unwrap();

        // Both proportion and fixed (proportion takes precedence)
        let both_table = lua.create_table().unwrap();
        both_table.set("proportion", 0.5).unwrap();
        both_table.set("fixed", 100i64).unwrap();
        let both_result = extract_size_change(&both_table).unwrap();

        // Zero values
        let zero_proportion_table = lua.create_table().unwrap();
        zero_proportion_table.set("proportion", 0.0).unwrap();
        let zero_proportion_result = extract_size_change(&zero_proportion_table).unwrap();

        let zero_fixed_table = lua.create_table().unwrap();
        zero_fixed_table.set("fixed", 0i64).unwrap();
        let zero_fixed_result = extract_size_change(&zero_fixed_table).unwrap();

        insta::assert_debug_snapshot!(
            "extractors_size_parsing_edge_cases",
            (
                empty_result,
                both_result,
                zero_proportion_result,
                zero_fixed_result,
            )
        );
    }

    #[test]
    fn snapshot_extractors_type_coercion_boundaries() {
        let (_lua, table) = create_test_table();

        // Test integer extraction from very large float
        table.set("large_float", 1e10).unwrap();
        let large_int = extract_int_opt(&table, "large_float").unwrap();

        // Test integer extraction from negative float
        table.set("neg_float", -123.456).unwrap();
        let neg_int = extract_int_opt(&table, "neg_float").unwrap();

        // Test float extraction from large integer
        table.set("large_int", 9007199254740992i64).unwrap();
        let large_float = extract_float_opt(&table, "large_int").unwrap();

        insta::assert_debug_snapshot!(
            "extractors_type_coercion_boundaries",
            (large_int, neg_int, large_float,)
        );
    }
}
