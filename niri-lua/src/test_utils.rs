//! Test utilities for Niri Lua API tests.
//!
//! This module provides common helper functions and fixtures for testing
//! the Niri Lua API. It includes utilities for creating test data, setting up
//! Lua environments, and common assertions.

#![cfg(test)]

use mlua::prelude::*;
use mlua::Result as LuaResult;
use niri_ipc::{Output, Transform, Window, WindowLayout, Workspace, LogicalOutput};

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

/// Helper to create a test Output without logical properties (disabled)
pub fn create_disabled_test_output(name: &str) -> Output {
    let mut output = create_test_output(name);
    output.logical = None;
    output.current_mode = None;
    output
}

/// Helper to create a Lua value from a string
pub fn lua_string(lua: &Lua, value: &str) -> mlua::Value {
    mlua::Value::String(lua.create_string(value).unwrap())
}

/// Helper to create a Lua value from a number
pub fn lua_number(value: f64) -> mlua::Value {
    mlua::Value::Number(value)
}

/// Helper to create a Lua value from an integer
pub fn lua_integer(value: i64) -> mlua::Value {
    mlua::Value::Integer(value)
}

/// Helper to create a Lua value from a boolean
pub fn lua_bool(value: bool) -> mlua::Value {
    mlua::Value::Boolean(value)
}

/// Helper to set up a Lua runtime with standard libraries
pub fn create_test_runtime() -> LuaResult<Lua> {
    let lua = Lua::new();
    lua.load_std_libs(mlua::prelude::LuaStdLib::ALL_SAFE)?;
    Ok(lua)
}

/// Helper to load and run Lua code in a test environment
/// 
/// This function with #[track_caller] provides better error messages
/// when assertions fail by showing the caller's location.
#[track_caller]
pub fn load_lua_code(code: &str) -> LuaResult<Lua> {
    let lua = create_test_runtime()?;
    lua.load(code).exec()?;
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
    fn test_create_disabled_test_output() {
        let output = create_disabled_test_output("DP-1");
        assert_eq!(output.name, "DP-1");
        assert!(output.logical.is_none());
    }

    #[test]
    fn test_lua_helpers() {
        let lua = Lua::new();
        let _str = lua_string(&lua, "test");
        let _num = lua_number(3.14);
        let _int = lua_integer(42);
        let _bool = lua_bool(true);
    }

    #[test]
    fn test_create_test_runtime() {
        let lua = create_test_runtime().unwrap();
        // Verify that standard library is loaded
        assert!(lua.globals().get::<mlua::Table>("math").is_ok());
    }

    #[test]
    fn test_load_lua_code() {
        let lua = load_lua_code("x = 42").unwrap();
        let x: i32 = lua.globals().get("x").unwrap();
        assert_eq!(x, 42);
    }

    #[test]
    fn test_get_lua_global() {
        let lua = load_lua_code("y = 'hello'").unwrap();
        let y: String = get_lua_global(&lua, "y").unwrap();
        assert_eq!(y, "hello");
    }
}
