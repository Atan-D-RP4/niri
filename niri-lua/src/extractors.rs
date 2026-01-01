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

/// Extract clipboard configuration from Lua value using serde.
pub fn extract_clipboard(lua: &Lua, value: LuaValue) -> LuaResult<Option<Clipboard>> {
    if value.is_nil() {
        return Ok(None);
    }
    lua.from_value(value).map(Some)
}

/// Extract overview configuration from Lua value using serde.
pub fn extract_overview(lua: &Lua, value: LuaValue) -> LuaResult<Option<Overview>> {
    if value.is_nil() {
        return Ok(None);
    }
    lua.from_value(value).map(Some)
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

// ============================================================================
// Output configuration extractors
// ============================================================================

// ============================================================================
// Layout configuration extractors (serde-based)
// ============================================================================

pub fn extract_layout(lua: &Lua, value: LuaValue) -> LuaResult<Option<Layout>> {
    if value.is_nil() {
        return Ok(None);
    }
    lua.from_value(value).map(Some)
}

pub fn extract_focus_ring(lua: &Lua, value: LuaValue) -> LuaResult<Option<FocusRing>> {
    if value.is_nil() {
        return Ok(None);
    }
    lua.from_value(value).map(Some)
}

pub fn extract_border(lua: &Lua, value: LuaValue) -> LuaResult<Option<Border>> {
    if value.is_nil() {
        return Ok(None);
    }
    lua.from_value(value).map(Some)
}

pub fn extract_shadow(lua: &Lua, value: LuaValue) -> LuaResult<Option<Shadow>> {
    if value.is_nil() {
        return Ok(None);
    }
    lua.from_value(value).map(Some)
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
// Gestures configuration extractors (serde-based)
// ============================================================================

pub fn extract_gestures(lua: &Lua, value: LuaValue) -> LuaResult<Option<Gestures>> {
    if value.is_nil() {
        return Ok(None);
    }
    lua.from_value(value).map(Some)
}

// ============================================================================
// Recent Windows (MRU) configuration extractors (serde-based)
// ============================================================================

pub fn extract_mru_highlight(lua: &Lua, value: LuaValue) -> LuaResult<Option<MruHighlight>> {
    if value.is_nil() {
        return Ok(None);
    }
    lua.from_value(value).map(Some)
}

pub fn extract_mru_previews(lua: &Lua, value: LuaValue) -> LuaResult<Option<MruPreviews>> {
    if value.is_nil() {
        return Ok(None);
    }
    lua.from_value(value).map(Some)
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
// Debug configuration extractors (serde-based)
// ============================================================================

pub fn extract_debug(lua: &Lua, value: LuaValue) -> LuaResult<Option<Debug>> {
    if value.is_nil() {
        return Ok(None);
    }
    lua.from_value(value).map(Some)
}

// ============================================================================
// Xwayland Satellite extractors (serde-based)
// ============================================================================

pub fn extract_xwayland_satellite(
    lua: &Lua,
    value: LuaValue,
) -> LuaResult<Option<XwaylandSatellite>> {
    if value.is_nil() {
        return Ok(None);
    }
    lua.from_value(value).map(Some)
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
        let (lua, table) = create_test_table();
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

        let result = extract_debug(&lua, LuaValue::Table(table)).unwrap();
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
        let (lua, table) = create_test_table();
        table.set("disable_primary", true).unwrap();
        let result = extract_clipboard(&lua, LuaValue::Table(table.clone())).unwrap();
        assert!(result.is_some());
        assert!(result.unwrap().disable_primary);
    }

    #[test]
    fn extract_clipboard_with_false() {
        let (lua, table) = create_test_table();
        table.set("disable_primary", false).unwrap();
        let result = extract_clipboard(&lua, LuaValue::Table(table.clone())).unwrap();
        assert!(result.is_some());
        assert!(!result.unwrap().disable_primary);
    }

    #[test]
    fn extract_clipboard_empty() {
        let (lua, table) = create_test_table();
        let result = extract_clipboard(&lua, LuaValue::Table(table.clone())).unwrap();
        // With serde #[serde(default)], empty table deserializes to default struct
        assert!(result.is_some());
        assert!(!result.unwrap().disable_primary); // default is false
    }

    // ========================================================================
    // extract_overview tests
    // ========================================================================

    #[test]
    fn extract_overview_with_zoom() {
        let (lua, table) = create_test_table();
        table.set("zoom", 0.5).unwrap();
        let result = extract_overview(&lua, LuaValue::Table(table.clone())).unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap().zoom, 0.5);
    }

    #[test]
    fn extract_overview_with_backdrop_color() {
        let (lua, table) = create_test_table();
        table.set("backdrop_color", "#000000").unwrap();
        let result = extract_overview(&lua, LuaValue::Table(table.clone())).unwrap();
        assert!(result.is_some());
    }

    #[test]
    fn extract_overview_with_both() {
        let (lua, table) = create_test_table();
        table.set("zoom", 0.7).unwrap();
        table.set("backdrop_color", "#111111").unwrap();
        let result = extract_overview(&lua, LuaValue::Table(table.clone())).unwrap();
        assert!(result.is_some());
        let overview = result.unwrap();
        assert_eq!(overview.zoom, 0.7);
    }

    #[test]
    fn extract_overview_empty() {
        let (lua, table) = create_test_table();
        let result = extract_overview(&lua, LuaValue::Table(table.clone())).unwrap();
        // With serde #[serde(default)], empty table deserializes to default struct
        assert!(result.is_some());
        let overview = result.unwrap();
        assert_eq!(overview.zoom, 0.5); // default zoom
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

        let result = extract_xkb(&lua, LuaValue::Table(table.clone())).unwrap();
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

        let result = extract_touchpad(&lua, LuaValue::Table(table.clone())).unwrap();
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
        let (lua, table) = create_test_table();
        table.set("off", false).unwrap();
        table.set("width", 4.0).unwrap();
        table.set("active_color", "#FF0000").unwrap();
        table.set("inactive_color", "#888888").unwrap();

        let result = extract_focus_ring(&lua, LuaValue::Table(table)).unwrap();
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
