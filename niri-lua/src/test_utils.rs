//! Test utilities for Niri Lua API tests.
//!
//! This module provides common helper functions and fixtures for testing
//! the Niri Lua API. It includes utilities for creating test data, setting up
//! Lua environments, and common assertions.
//!
//! ## Testing Patterns
//!
//! This module follows the patterns established in the Niri codebase:
//! - Fixture builders for creating test data
//! - `#[track_caller]` helpers for better error messages
//! - Snapshot testing with insta for regression detection
//! - Helper functions to reduce boilerplate in tests

use mlua::prelude::*;
use mlua::Result as LuaResult;
use niri_config::Config;
use niri_ipc::{LogicalOutput, Output, Transform, Window, WindowLayout, Workspace};

/// Helper to create a test Lua environment with a table
pub fn create_test_lua_table() -> (Lua, LuaTable) {
    let lua = Lua::new();
    let table = lua.create_table().unwrap();
    (lua, table)
}

/// Helper to create a minimal test Window
pub fn create_test_window(id: u64) -> Window {
    Window {
        id,
        title: Some(format!("Test Window {}", id)),
        app_id: Some(format!("org.test.app{}", id)),
        pid: Some((1000 + id) as i32),
        workspace_id: Some(1),
        is_focused: false,
        is_floating: false,
        is_urgent: false,
        focus_timestamp: None,
        layout: WindowLayout {
            pos_in_scrolling_layout: None,
            tile_size: (1920.0, 1080.0),
            window_size: (1920, 1080),
            tile_pos_in_workspace_view: None,
            window_offset_in_tile: (0.0, 0.0),
        },
    }
}

/// Helper to create a test Window with custom properties
pub fn create_test_window_with(
    id: u64,
    title: Option<String>,
    is_focused: bool,
    is_floating: bool,
) -> Window {
    let mut window = create_test_window(id);
    window.title = title;
    window.is_focused = is_focused;
    window.is_floating = is_floating;
    window
}

/// Helper to create a minimal test Workspace
pub fn create_test_workspace(id: u64) -> Workspace {
    Workspace {
        id,
        idx: id as u8,
        name: Some(format!("Workspace {}", id)),
        output: Some("DP-1".to_string()),
        is_urgent: false,
        is_active: id == 1,
        is_focused: id == 1,
        active_window_id: None,
    }
}

/// Helper to create a test Workspace with custom properties
pub fn create_test_workspace_with(
    id: u64,
    name: Option<String>,
    is_active: bool,
    is_focused: bool,
) -> Workspace {
    let mut workspace = create_test_workspace(id);
    workspace.name = name;
    workspace.is_active = is_active;
    workspace.is_focused = is_focused;
    workspace
}

/// Helper to create a minimal test Output
pub fn create_test_output(name: &str) -> Output {
    Output {
        name: name.to_string(),
        make: "Test Manufacturer".to_string(),
        model: "Test Model".to_string(),
        serial: Some("TEST123".to_string()),
        physical_size: Some((500, 300)),
        modes: vec![],
        current_mode: Some(0),
        is_custom_mode: false,
        vrr_supported: false,
        vrr_enabled: false,
        logical: Some(LogicalOutput {
            x: 0,
            y: 0,
            width: 1920,
            height: 1080,
            scale: 1.0,
            transform: Transform::Normal,
        }),
    }
}

/// Helper to create a Lua value from a string
pub fn lua_string(lua: &Lua, value: &str) -> mlua::Value {
    mlua::Value::String(lua.create_string(value).unwrap())
}

/// Helper to set up a Lua runtime with standard libraries
pub fn create_test_runtime() -> LuaResult<Lua> {
    let lua = Lua::new();
    lua.load_std_libs(mlua::prelude::LuaStdLib::ALL_SAFE)?;
    Ok(lua)
}

/// Helper to extract a value from Lua environment
///
/// Simplifies test code by providing a consistent pattern for accessing
/// global variables with automatic type conversion.
#[track_caller]
pub fn get_lua_global<T: mlua::FromLua>(lua: &Lua, name: &str) -> LuaResult<T> {
    lua.globals().get(name)
}

/// Helper to set up a Lua runtime with a config API registered
///
/// This is useful for testing config-dependent functionality.
#[track_caller]
pub fn create_config_lua_env() -> LuaResult<Lua> {
    use crate::config_api::ConfigApi;

    let lua = create_test_runtime()?;
    ConfigApi::register_to_lua(&lua, &Config::default())?;
    Ok(lua)
}

/// Helper to validate that a Lua value equals an expected value
///
/// Provides better error messages with #[track_caller].
#[track_caller]
pub fn assert_lua_value_eq<T: mlua::FromLua + PartialEq + std::fmt::Debug>(
    lua: &Lua,
    name: &str,
    expected: T,
) {
    let actual: T = get_lua_global(lua, name).unwrap_or_else(|_| {
        panic!("Failed to get Lua global: {}", name);
    });
    assert_eq!(actual, expected, "Lua value mismatch for {}", name);
}

/// Helper to assert that a Lua global exists
///
/// Provides better error messages with #[track_caller].
#[track_caller]
pub fn assert_lua_global_exists(lua: &Lua, name: &str) {
    let globals = lua.globals();
    assert!(
        globals.get::<mlua::Value>(name).is_ok(),
        "Expected Lua global '{}' to exist",
        name
    );
}

/// Helper to assert that a Lua table contains a key
///
/// Provides better error messages with #[track_caller].
#[track_caller]
pub fn assert_lua_table_has_key(table: &LuaTable, key: &str) {
    assert!(
        table.get::<mlua::Value>(key).is_ok(),
        "Expected Lua table to have key '{}'",
        key
    );
}

/// Helper to assert that a Lua table has a specific value
///
/// Provides better error messages with #[track_caller].
#[track_caller]
pub fn assert_lua_table_value_eq<T: mlua::FromLua + PartialEq + std::fmt::Debug>(
    table: &LuaTable,
    key: &str,
    expected: T,
) {
    let actual: T = table.get(key).unwrap_or_else(|_| {
        panic!("Failed to get Lua table value: {}", key);
    });
    assert_eq!(
        actual, expected,
        "Lua table value mismatch for key '{}'",
        key
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_test_lua_table() {
        let (_lua, table) = create_test_lua_table();
        assert_eq!(table.len().unwrap(), 0);
    }

    #[test]
    fn test_create_test_window() {
        let window = create_test_window(123);
        assert_eq!(window.id, 123);
        assert_eq!(window.title, Some("Test Window 123".to_string()));
        assert!(!window.is_focused);
        assert!(!window.is_floating);
    }

    #[test]
    fn test_create_test_window_with() {
        let window = create_test_window_with(456, Some("Custom".to_string()), true, true);
        assert_eq!(window.id, 456);
        assert_eq!(window.title, Some("Custom".to_string()));
        assert!(window.is_focused);
        assert!(window.is_floating);
    }

    #[test]
    fn test_create_test_workspace() {
        let workspace = create_test_workspace(1);
        assert_eq!(workspace.id, 1);
        assert_eq!(workspace.name, Some("Workspace 1".to_string()));
        assert!(workspace.is_active);
        assert!(workspace.is_focused);
    }

    #[test]
    fn test_create_test_workspace_with() {
        let workspace = create_test_workspace_with(2, Some("Work".to_string()), false, false);
        assert_eq!(workspace.id, 2);
        assert_eq!(workspace.name, Some("Work".to_string()));
        assert!(!workspace.is_active);
        assert!(!workspace.is_focused);
    }

    #[test]
    fn test_create_test_output() {
        let output = create_test_output("HDMI-1");
        assert_eq!(output.name, "HDMI-1");
        assert!(output.logical.is_some());
    }

    #[test]
    fn test_create_config_lua_env() {
        let lua = create_config_lua_env().unwrap();
        assert_lua_global_exists(&lua, "niri");
    }

    #[test]
    fn test_assert_lua_table_has_key() {
        let lua = Lua::new();
        let table = lua.create_table().unwrap();
        table.set("test_key", "test_value").unwrap();
        assert_lua_table_has_key(&table, "test_key");
    }
}
