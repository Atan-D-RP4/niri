//! Lua configuration value extractors.
//!
//! This module provides utilities for extracting complex configuration structures
//! from Lua tables and converting them to Niri config types.

use mlua::prelude::*;
use niri_config::{
    animations::*, appearance::*, input::*, layout::*, misc::*, output::*, window_rule::*,
    binds::*, gestures::*,
};
use log::{debug, warn};
use std::str::FromStr;

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
pub fn extract_table_opt<'lua>(table: &'lua LuaTable, field: &str) -> LuaResult<Option<LuaTable<'lua>>> {
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
    let hide_after_inactive_ms = extract_int_opt(table, "hide_after_inactive_ms")?.map(|i| i as u32);
    
    if xcursor_theme.is_some() || xcursor_size.is_some() || hide_when_typing.is_some() || hide_after_inactive_ms.is_some() {
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
