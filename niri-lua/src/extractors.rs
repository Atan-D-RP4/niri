//! Lua configuration value extractors.
//!
//! This module provides utilities for extracting complex configuration structures
//! from Lua tables and converting them to Niri config types.

use std::str::FromStr;

use mlua::prelude::*;
use niri_config::animations::*;
use niri_config::appearance::*;
use niri_config::misc::*;

/// Helper to extract an optional string field from a Lua table.
pub fn extract_string_opt(table: &LuaTable, field: &str) -> LuaResult<Option<String>> {
    match table.get::<LuaValue>(field) {
        Ok(LuaValue::Nil) => Ok(None),
        Ok(LuaValue::String(s)) => Ok(Some(s.to_string_lossy().to_string())),
        Ok(_) => Ok(None),
        Err(_) => Ok(None),
    }
}

/// Helper to extract an optional boolean field from a Lua table.
pub fn extract_bool_opt(table: &LuaTable, field: &str) -> LuaResult<Option<bool>> {
    match table.get::<LuaValue>(field) {
        Ok(LuaValue::Nil) => Ok(None),
        Ok(LuaValue::Boolean(b)) => Ok(Some(b)),
        Ok(_) => Ok(None),
        Err(_) => Ok(None),
    }
}

/// Helper to extract an optional integer field from a Lua table.
pub fn extract_int_opt(table: &LuaTable, field: &str) -> LuaResult<Option<i64>> {
    match table.get::<LuaValue>(field) {
        Ok(LuaValue::Nil) => Ok(None),
        Ok(LuaValue::Integer(i)) => Ok(Some(i)),
        Ok(LuaValue::Number(n)) => Ok(Some(n as i64)),
        Ok(_) => Ok(None),
        Err(_) => Ok(None),
    }
}

/// Helper to extract an optional float field from a Lua table.
pub fn extract_float_opt(table: &LuaTable, field: &str) -> LuaResult<Option<f64>> {
    match table.get::<LuaValue>(field) {
        Ok(LuaValue::Nil) => Ok(None),
        Ok(LuaValue::Number(n)) => Ok(Some(n)),
        Ok(LuaValue::Integer(i)) => Ok(Some(i as f64)),
        Ok(_) => Ok(None),
        Err(_) => Ok(None),
    }
}

/// Helper to extract an optional table field from a Lua table.
pub fn extract_table_opt<'lua>(table: &'lua LuaTable, field: &str) -> LuaResult<Option<LuaTable>> {
    match table.get::<LuaValue>(field) {
        Ok(LuaValue::Nil) => Ok(None),
        Ok(LuaValue::Table(t)) => Ok(Some(t)),
        Ok(_) => Ok(None),
        Err(_) => Ok(None),
    }
}

/// Extract color from a Lua string in hex format (#RGB, #RGBA, #RRGGBB, #RRGGBBAA).
pub fn extract_color(color_str: &str) -> Option<Color> {
    Color::from_str(color_str).ok()
}

/// Extract a color from an optional Lua table field.
pub fn extract_color_opt(table: &LuaTable, field: &str) -> LuaResult<Option<Color>> {
    if let Some(color_str) = extract_string_opt(table, field)? {
        Ok(extract_color(&color_str))
    } else {
        Ok(None)
    }
}

/// Extract screenshot path configuration from Lua table.
pub fn extract_screenshot_path(table: &LuaTable) -> LuaResult<Option<ScreenshotPath>> {
    if let Some(path) = extract_string_opt(table, "path")? {
        Ok(Some(ScreenshotPath(Some(path))))
    } else if let Some(false) = extract_bool_opt(table, "path")? {
        // path = false or nil means disable
        Ok(Some(ScreenshotPath(None)))
    } else {
        Ok(None)
    }
}

/// Extract hotkey overlay configuration from Lua table.
pub fn extract_hotkey_overlay(table: &LuaTable) -> LuaResult<Option<HotkeyOverlay>> {
    let skip_at_startup = extract_bool_opt(table, "skip_at_startup")?.unwrap_or(false);
    let hide_not_bound = extract_bool_opt(table, "hide_not_bound")?.unwrap_or(false);

    if skip_at_startup || hide_not_bound {
        Ok(Some(HotkeyOverlay {
            skip_at_startup,
            hide_not_bound,
        }))
    } else {
        Ok(None)
    }
}

/// Extract cursor configuration from Lua table.
pub fn extract_cursor(table: &LuaTable) -> LuaResult<Option<Cursor>> {
    let xcursor_theme = extract_string_opt(table, "xcursor_theme")?;
    let xcursor_size = extract_int_opt(table, "xcursor_size")?.map(|i| i as u8);
    let hide_when_typing = extract_bool_opt(table, "hide_when_typing")?;
    let hide_after_inactive_ms =
        extract_int_opt(table, "hide_after_inactive_ms")?.map(|i| i as u32);

    if xcursor_theme.is_some()
        || xcursor_size.is_some()
        || hide_when_typing.is_some()
        || hide_after_inactive_ms.is_some()
    {
        let mut cursor = Cursor::default();
        if let Some(theme) = xcursor_theme {
            cursor.xcursor_theme = theme;
        }
        if let Some(size) = xcursor_size {
            cursor.xcursor_size = size;
        }
        if let Some(hide) = hide_when_typing {
            cursor.hide_when_typing = hide;
        }
        cursor.hide_after_inactive_ms = hide_after_inactive_ms;
        Ok(Some(cursor))
    } else {
        Ok(None)
    }
}

/// Extract animations configuration from Lua table.
pub fn extract_animations(table: &LuaTable) -> LuaResult<Option<Animations>> {
    let off = extract_bool_opt(table, "off")?.unwrap_or(false);
    let on = extract_bool_opt(table, "on")?.unwrap_or(false);
    let slowdown = extract_float_opt(table, "slowdown")?;

    if off || on || slowdown.is_some() {
        let mut animations = Animations::default();
        animations.off = off && !on; // on overrides off
        if let Some(s) = slowdown {
            animations.slowdown = s;
        }
        // TODO: Extract individual animation settings
        Ok(Some(animations))
    } else {
        Ok(None)
    }
}

/// Extract simple boolean flags like prefer_no_csd.
pub fn extract_prefer_no_csd(table: &LuaTable) -> LuaResult<Option<bool>> {
    extract_bool_opt(table, "prefer_no_csd")
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

// TODO: Add more complex extractors for:
// - Input configuration (keyboard, touchpad, mouse, etc.)
// - Layout configuration (focus_ring, border, struts, etc.)
// - Output configuration
// - Window rules
// - Layer rules
// - Workspaces
// - Gestures
// - Switch Events
// - Recent Windows
// - Debug options
// - Named Workspaces
// - Miscellaneous settings

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::create_test_lua_table;

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

    #[test]
    fn extract_screenshot_path_with_string() {
        let (_lua, table) = create_test_table();
        table.set("path", "/home/user/screenshots").unwrap();
        let result = extract_screenshot_path(&table).unwrap();
        assert!(result.is_some());
        assert_eq!(
            result.unwrap().0,
            Some("/home/user/screenshots".to_string())
        );
    }

    #[test]
    fn extract_screenshot_path_disabled_with_false() {
        let (_lua, table) = create_test_table();
        table.set("path", false).unwrap();
        let result = extract_screenshot_path(&table).unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap().0, None);
    }

    #[test]
    fn extract_screenshot_path_missing() {
        let (_lua, table) = create_test_table();
        let result = extract_screenshot_path(&table).unwrap();
        assert_eq!(result, None);
    }

    // ========================================================================
    // extract_hotkey_overlay tests
    // ========================================================================

    #[test]
    fn extract_hotkey_overlay_with_values() {
        let (_lua, table) = create_test_table();
        table.set("skip_at_startup", true).unwrap();
        table.set("hide_not_bound", false).unwrap();
        let result = extract_hotkey_overlay(&table).unwrap();
        assert!(result.is_some());
        let overlay = result.unwrap();
        assert_eq!(overlay.skip_at_startup, true);
        assert_eq!(overlay.hide_not_bound, false);
    }

    #[test]
    fn extract_hotkey_overlay_both_true() {
        let (_lua, table) = create_test_table();
        table.set("skip_at_startup", true).unwrap();
        table.set("hide_not_bound", true).unwrap();
        let result = extract_hotkey_overlay(&table).unwrap();
        assert!(result.is_some());
        let overlay = result.unwrap();
        assert_eq!(overlay.skip_at_startup, true);
        assert_eq!(overlay.hide_not_bound, true);
    }

    #[test]
    fn extract_hotkey_overlay_empty() {
        let (_lua, table) = create_test_table();
        let result = extract_hotkey_overlay(&table).unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn extract_hotkey_overlay_both_false() {
        let (_lua, table) = create_test_table();
        table.set("skip_at_startup", false).unwrap();
        table.set("hide_not_bound", false).unwrap();
        let result = extract_hotkey_overlay(&table).unwrap();
        assert_eq!(result, None);
    }

    // ========================================================================
    // extract_cursor tests
    // ========================================================================

    #[test]
    fn extract_cursor_with_all_fields() {
        let (_lua, table) = create_test_table();
        table.set("xcursor_theme", "default").unwrap();
        table.set("xcursor_size", 32i64).unwrap();
        table.set("hide_when_typing", true).unwrap();
        table.set("hide_after_inactive_ms", 1000i64).unwrap();

        let result = extract_cursor(&table).unwrap();
        assert!(result.is_some());
        let cursor = result.unwrap();
        assert_eq!(cursor.xcursor_theme, "default");
        assert_eq!(cursor.xcursor_size, 32);
        assert_eq!(cursor.hide_when_typing, true);
        assert_eq!(cursor.hide_after_inactive_ms, Some(1000));
    }

    #[test]
    fn extract_cursor_partial_fields() {
        let (_lua, table) = create_test_table();
        table.set("xcursor_theme", "default").unwrap();

        let result = extract_cursor(&table).unwrap();
        assert!(result.is_some());
        let cursor = result.unwrap();
        assert_eq!(cursor.xcursor_theme, "default");
    }

    #[test]
    fn extract_cursor_empty() {
        let (_lua, table) = create_test_table();
        let result = extract_cursor(&table).unwrap();
        assert_eq!(result, None);
    }

    // ========================================================================
    // extract_animations tests
    // ========================================================================

    #[test]
    fn extract_animations_with_off() {
        let (_lua, table) = create_test_table();
        table.set("off", true).unwrap();
        let result = extract_animations(&table).unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap().off, true);
    }

    #[test]
    fn extract_animations_with_slowdown() {
        let (_lua, table) = create_test_table();
        table.set("slowdown", 2.0).unwrap();
        let result = extract_animations(&table).unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap().slowdown, 2.0);
    }

    #[test]
    fn extract_animations_on_overrides_off() {
        let (_lua, table) = create_test_table();
        table.set("off", true).unwrap();
        table.set("on", true).unwrap();
        let result = extract_animations(&table).unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap().off, false); // on should override off
    }

    #[test]
    fn extract_animations_empty() {
        let (_lua, table) = create_test_table();
        let result = extract_animations(&table).unwrap();
        assert_eq!(result, None);
    }

    // ========================================================================
    // extract_prefer_no_csd tests
    // ========================================================================

    #[test]
    fn extract_prefer_no_csd_true() {
        let (_lua, table) = create_test_table();
        table.set("prefer_no_csd", true).unwrap();
        let result = extract_prefer_no_csd(&table).unwrap();
        assert_eq!(result, Some(true));
    }

    #[test]
    fn extract_prefer_no_csd_false() {
        let (_lua, table) = create_test_table();
        table.set("prefer_no_csd", false).unwrap();
        let result = extract_prefer_no_csd(&table).unwrap();
        assert_eq!(result, Some(false));
    }

    // ========================================================================
    // extract_clipboard tests
    // ========================================================================

    #[test]
    fn extract_clipboard_with_true() {
        let (_lua, table) = create_test_table();
        table.set("disable_primary", true).unwrap();
        let result = extract_clipboard(&table).unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap().disable_primary, true);
    }

    #[test]
    fn extract_clipboard_with_false() {
        let (_lua, table) = create_test_table();
        table.set("disable_primary", false).unwrap();
        let result = extract_clipboard(&table).unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap().disable_primary, false);
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
}
