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

#![cfg(test)]

use std::collections::HashMap;

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

/// Helper to set up a Lua runtime with a niri API registered
///
/// This is useful for testing niri-dependent functionality.
/// Note: This requires a callback function for niri actions.
#[track_caller]
pub fn create_niri_lua_env_with_callback<F>(callback: F) -> LuaResult<Lua>
where
    F: Fn(String, Vec<String>) -> LuaResult<()> + 'static,
{
    use crate::niri_api::NiriApi;
    use crate::LuaComponent;

    let lua = create_test_runtime()?;
    NiriApi::register_to_lua(&lua, callback)?;
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

/// Helper to build test data with a builder pattern
///
/// This fixture builder provides a fluent API for constructing test data.
pub struct TestDataBuilder {
    config: Config,
    windows: Vec<Window>,
    workspaces: Vec<Workspace>,
    outputs: Vec<Output>,
}

impl TestDataBuilder {
    /// Create a new test data builder with default config
    pub fn new() -> Self {
        Self {
            config: Config::default(),
            windows: Vec::new(),
            workspaces: Vec::new(),
            outputs: Vec::new(),
        }
    }

    /// Add a window to the test data
    pub fn with_window(mut self, window: Window) -> Self {
        self.windows.push(window);
        self
    }

    /// Add a workspace to the test data
    pub fn with_workspace(mut self, workspace: Workspace) -> Self {
        self.workspaces.push(workspace);
        self
    }

    /// Add an output to the test data
    pub fn with_output(mut self, output: Output) -> Self {
        self.outputs.push(output);
        self
    }

    /// Get the config
    pub fn config(&self) -> &Config {
        &self.config
    }

    /// Get the windows
    pub fn windows(&self) -> &[Window] {
        &self.windows
    }

    /// Get the workspaces
    pub fn workspaces(&self) -> &[Workspace] {
        &self.workspaces
    }

    /// Get the outputs
    pub fn outputs(&self) -> &[Output] {
        &self.outputs
    }
}

impl Default for TestDataBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper to extract table as a HashMap for snapshot testing
///
/// Useful for comparing complex Lua table structures in snapshots.
#[track_caller]
pub fn lua_table_to_map(table: &LuaTable) -> LuaResult<HashMap<String, String>> {
    let mut map = HashMap::new();

    for pair in table.pairs::<String, mlua::Value>() {
        let (key, value) = pair?;
        let value_str = format!("{:?}", value);
        map.insert(key, value_str);
    }

    Ok(map)
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

    #[test]
    fn test_create_config_lua_env() {
        let lua = create_config_lua_env().unwrap();
        assert_lua_global_exists(&lua, "niri");
    }

    #[test]
    fn test_create_niri_lua_env_with_callback() {
        let lua = create_niri_lua_env_with_callback(|_, _| Ok(())).unwrap();
        assert_lua_global_exists(&lua, "niri");
    }

    #[test]
    fn test_assert_lua_table_has_key() {
        let lua = Lua::new();
        let table = lua.create_table().unwrap();
        table.set("test_key", "test_value").unwrap();
        assert_lua_table_has_key(&table, "test_key");
    }

    #[test]
    fn test_test_data_builder() {
        let builder = TestDataBuilder::new()
            .with_window(create_test_window(1))
            .with_workspace(create_test_workspace(1))
            .with_output(create_test_output("HDMI-1"));

        assert_eq!(builder.windows().len(), 1);
        assert_eq!(builder.workspaces().len(), 1);
        assert_eq!(builder.outputs().len(), 1);
    }

    #[test]
    fn test_lua_table_to_map() {
        let lua = Lua::new();
        let table = lua.create_table().unwrap();
        table.set("key1", "value1").unwrap();
        table.set("key2", 42).unwrap();

        let map = lua_table_to_map(&table).unwrap();
        assert_eq!(map.len(), 2);
        assert!(map.contains_key("key1"));
        assert!(map.contains_key("key2"));
    }
}
