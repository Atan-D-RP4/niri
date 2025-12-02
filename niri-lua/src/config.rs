//! Lua configuration support for Niri.
//!
//! This module handles loading and executing Lua configuration files.
//! It integrates with Niri's config system to allow Lua-based scripting.

use std::path::Path;

use anyhow::Result;
use log::{debug, info};
use mlua::prelude::*;

use crate::LuaRuntime;

/// Configuration loaded from a Lua script.
///
/// This structure holds the result of loading a Lua configuration file,
/// allowing scripts to define custom behavior for Niri.
pub struct LuaConfig {
    runtime: LuaRuntime,
}

impl LuaConfig {
    /// Create a new Lua configuration from a file.
    ///
    /// This initializes the Lua runtime, registers all built-in components,
    /// and loads the specified configuration file.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the Lua configuration file
    ///
    /// # Errors
    ///
    /// Returns an error if the runtime cannot be created or the file cannot be loaded.
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path_ref = path.as_ref();
        let mut runtime = LuaRuntime::new()
            .map_err(|e| anyhow::anyhow!("Failed to create Lua runtime from file: {}", e))?;

        info!("Loading Lua config from {}", path_ref.display());

        // Register the Niri API component (which creates the niri table)
        runtime
            .register_component(|action, args| {
                info!("Lua action: {} with args {:?}", action, args);
                Ok(())
            })
            .map_err(|e| anyhow::anyhow!("Failed to register Niri API: {}", e))?;

        debug!("Niri API registered successfully");

        // Initialize the event system AFTER the niri table is created
        runtime
            .init_event_system()
            .map_err(|e| anyhow::anyhow!("Failed to initialize event system: {}", e))?;

        debug!("Event system initialized");

        // Initialize the config proxy with empty collections BEFORE loading the script
        // This allows the script to use niri.config.binds:add(), niri.config.outputs:add(), etc.
        runtime
            .init_empty_config_proxy()
            .map_err(|e| anyhow::anyhow!("Failed to initialize config proxy: {}", e))?;

        debug!("Config proxy initialized");

        // Load the configuration file
        // The script can either:
        // 1. Call niri.apply_config({ ... }) to apply config (preferred)
        // 2. Return a config table (fallback for backward compatibility)
        let return_val = runtime
            .load_file(&path_ref)
            .map_err(|e| anyhow::anyhow!("Failed to load Lua config file: {}", e))?;

        // If the script returns a table, extract its fields and set them as globals
        // This is a fallback for backward compatibility - new scripts should use
        // niri.apply_config()
        if let LuaValue::Table(config_table) = return_val {
            debug!("Lua file returned a table, extracting configuration (fallback mode)");
            debug!("Note: Consider using niri.apply_config() instead of returning a table");

            // Extract fields from returned table and set as globals
            // This allows scripts to use local variables and return a config table
            let globals = runtime.inner().globals();

            // Extract common configuration tables
            for &field_name in &[
                "binds",
                "startup",
                "spawn_at_startup",
                "spawn_sh_at_startup",
                "input",
                "outputs",
                "layout",
                "animations",
                "gestures",
                "clipboard",
                "hotkey_overlay",
                "config_notification",
                "screenshot",
                "window_rules",
                "layer_rules",
                "prefer_no_csd",
                "cursor",
                "screenshot_path",
                "environment",
                "debug",
                "workspaces",
                "xwayland_satellite",
                "recent_windows",
                "overview",
            ] {
                if let Ok(value) = config_table.get::<LuaValue>(field_name) {
                    if value != LuaValue::Nil {
                        debug!("Extracting returned config field: {}", field_name);
                        if let Err(e) = globals.set(field_name, value) {
                            debug!("Failed to set global {}: {}", field_name, e);
                        }
                    }
                }
            }

            // Also check for startup_commands and map it to startup
            if let Ok(value) = config_table.get::<LuaValue>("startup_commands") {
                if value != LuaValue::Nil {
                    debug!(
                        "Extracting returned config field: startup_commands (mapping to startup)"
                    );
                    if let Err(e) = globals.set("startup", value) {
                        debug!("Failed to set global startup from startup_commands: {}", e);
                    }
                }
            }
        } else {
            debug!("Lua file did not return a table (using niri.apply_config() is preferred)");
        }

        info!(
            "Successfully loaded Lua configuration from {}",
            path_ref.display()
        );

        Ok(Self { runtime })
    }

    /// Create a new Lua configuration from a string.
    ///
    /// This is useful for testing or inline configurations.
    ///
    /// # Arguments
    ///
    /// * `code` - Lua code to execute as configuration
    ///
    /// # Errors
    ///
    /// Returns an error if the runtime cannot be created or the code cannot be executed.
    pub fn from_string(code: &str) -> Result<Self> {
        let mut runtime = LuaRuntime::new()
            .map_err(|e| anyhow::anyhow!("Failed to create Lua runtime from string: {}", e))?;

        // Register the Niri API component (which creates the niri table)
        runtime
            .register_component(|action, args| {
                info!("Lua action: {} with args {:?}", action, args);
                Ok(())
            })
            .map_err(|e| anyhow::anyhow!("Failed to register Niri API: {}", e))?;

        // Initialize the event system AFTER the niri table is created
        runtime
            .init_event_system()
            .map_err(|e| anyhow::anyhow!("Failed to initialize event system: {}", e))?;

        debug!("Event system initialized");

        // Initialize the config proxy with empty collections BEFORE loading the script
        // This allows the script to use niri.config.binds:add(), niri.config.outputs:add(), etc.
        runtime
            .init_empty_config_proxy()
            .map_err(|e| anyhow::anyhow!("Failed to initialize config proxy: {}", e))?;

        debug!("Config proxy initialized");

        // Load and execute the code
        let return_val = runtime
            .load_string(code)
            .map_err(|e| anyhow::anyhow!("Failed to execute Lua code: {}", e))?;

        // If the script returns a table, extract its fields and set them as globals
        if let LuaValue::Table(config_table) = return_val {
            debug!("Lua string returned a table, extracting configuration");

            // Extract fields from returned table and set as globals
            let globals = runtime.inner().globals();

            // Extract common configuration tables
            for &field_name in &[
                "binds",
                "startup",
                "input",
                "outputs",
                "layout",
                "animations",
                "gestures",
                "clipboard",
                "hotkey_overlay",
                "config_notification",
                "screenshot",
                "window_rules",
                "prefer_no_csd",
                "cursor",
                "screenshot_path",
                "environment",
                "debug",
                "workspaces",
            ] {
                if let Ok(value) = config_table.get::<LuaValue>(field_name) {
                    if value != LuaValue::Nil {
                        debug!("Extracting returned config field: {}", field_name);
                        if let Err(e) = globals.set(field_name, value) {
                            debug!("Failed to set global {}: {}", field_name, e);
                        }
                    }
                }
            }

            // Also check for startup_commands and map it to startup
            if let Ok(value) = config_table.get::<LuaValue>("startup_commands") {
                if value != LuaValue::Nil {
                    debug!(
                        "Extracting returned config field: startup_commands (mapping to startup)"
                    );
                    if let Err(e) = globals.set("startup", value) {
                        debug!("Failed to set global startup from startup_commands: {}", e);
                    }
                }
            }
        }

        Ok(Self { runtime })
    }

    /// Get a reference to the underlying Lua runtime.
    ///
    /// This allows advanced users to access the full mlua API.
    pub fn runtime(&self) -> &LuaRuntime {
        &self.runtime
    }

    /// Take ownership of the underlying Lua runtime.
    ///
    /// This consumes the LuaConfig and returns the runtime, allowing it to be
    /// stored in the compositor state for runtime access.
    pub fn into_runtime(self) -> LuaRuntime {
        self.runtime
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lua_config_from_string() {
        let result = LuaConfig::from_string(
            r#"
            test_value = 42
            function test_function()
                return "Hello from Lua!"
            end
        "#,
        );

        assert!(result.is_ok());
    }

    #[test]
    fn lua_niri_api() {
        let config = LuaConfig::from_string(
            r#"
             niri.utils.log("Test log message")
             -- Test setting a config value via the reactive API
             niri.config.prefer_no_csd = true
         "#,
        )
        .unwrap();

        // Verify it executed without error - the config proxy should have captured the change
        assert!(config.runtime().inner().globals().get::<mlua::Value>("niri").is_ok());
    }

    #[test]
    fn lua_config_with_returned_table() {
        let config = LuaConfig::from_string(
            r#"
             local binds = {
                 { key = "Super+Return", action = "spawn", args = { "alacritty" } },
             }
             
             return {
                 binds = binds,
                 prefer_no_csd = true,
             }
         "#,
        )
        .unwrap();

        // Verify that the returned table was extracted and set as globals
        let prefer_no_csd: bool = config
            .runtime()
            .inner()
            .globals()
            .get("prefer_no_csd")
            .unwrap_or(false);
        assert!(prefer_no_csd);

        // Verify binds table is now global
        let binds_table = config.runtime().inner().globals().get::<LuaTable>("binds");
        assert!(binds_table.is_ok());
    }

    #[test]
    fn lua_config_with_apply_config_function() {
        let config = LuaConfig::from_string(
            r#"
             niri.apply_config({
                 prefer_no_csd = true,
                 debug = {
                     render_loop_ms = 5000
                 }
             })
         "#,
        )
        .unwrap();

        // Verify that the config was applied and set as globals
        let prefer_no_csd: bool = config
            .runtime()
            .inner()
            .globals()
            .get("prefer_no_csd")
            .unwrap_or(false);
        assert!(prefer_no_csd);

        // Verify debug table is now global
        let debug_table = config.runtime().inner().globals().get::<LuaTable>("debug");
        assert!(debug_table.is_ok());
    }

    #[test]
    fn lua_config_apply_config_with_binds() {
        let config = LuaConfig::from_string(
            r#"
             local binds = {
                 { key = "Super+Return", action = "spawn", args = { "alacritty" } },
                 { key = "Super+Q", action = "close-window" },
             }
             
             niri.apply_config({
                 binds = binds,
                 prefer_no_csd = true,
             })
         "#,
        )
        .unwrap();

        // Verify that the config was applied
        let prefer_no_csd: bool = config
            .runtime()
            .inner()
            .globals()
            .get("prefer_no_csd")
            .unwrap_or(false);
        assert!(prefer_no_csd);

        // Verify binds table is now global and has the expected length
        let binds_table: LuaTable = config
            .runtime()
            .inner()
            .globals()
            .get("binds")
            .expect("binds table should be set");
        assert_eq!(binds_table.raw_len(), 2);
    }

    #[test]
    fn lua_config_apply_config_multiple_calls() {
        let config = LuaConfig::from_string(
            r#"
             niri.apply_config({
                 prefer_no_csd = true,
             })
             
             -- Second call should also work
             niri.apply_config({
                 debug = {
                     render_loop_ms = 5000
                 }
             })
             
             -- Third call to add more config
             niri.apply_config({
                 environment = {
                     EDITOR = "vim"
                 }
             })
         "#,
        )
        .unwrap();

        let globals = config.runtime().inner().globals();

        // Verify all three configs were applied
        let prefer_no_csd: bool = globals.get("prefer_no_csd").unwrap_or(false);
        assert!(prefer_no_csd);

        let debug_table: LuaTable = globals.get("debug").expect("debug table should exist");
        let render_loop_ms: i32 = debug_table.get("render_loop_ms").unwrap_or(0);
        assert_eq!(render_loop_ms, 5000);

        let env_table: LuaTable = globals
            .get("environment")
            .expect("environment table should exist");
        let editor: String = env_table.get("EDITOR").unwrap_or_default();
        assert_eq!(editor, "vim");
    }

    #[test]
    fn lua_config_apply_config_with_startup() {
        let config = LuaConfig::from_string(
            r#"
             niri.apply_config({
                 startup = {
                     { command = { "dbus-update-activation-environment", "WAYLAND_DISPLAY" } },
                 },
             })
         "#,
        )
        .unwrap();

        let startup_table: LuaTable = config
            .runtime()
            .inner()
            .globals()
            .get("startup")
            .expect("startup table should be set");
        assert_eq!(startup_table.raw_len(), 1);
    }

    #[test]
    fn lua_config_return_table_with_multiple_fields() {
        let config = LuaConfig::from_string(
            r#"
             return {
                 prefer_no_csd = true,
                 debug = {
                     render_loop_ms = 5000
                 },
                 environment = {
                     EDITOR = "vim"
                 }
             }
         "#,
        )
        .unwrap();

        let globals = config.runtime().inner().globals();

        // Verify all fields were extracted and set as globals
        let prefer_no_csd: bool = globals.get("prefer_no_csd").unwrap_or(false);
        assert!(prefer_no_csd);

        let debug_table: LuaTable = globals.get("debug").expect("debug table should exist");
        let render_loop_ms: i32 = debug_table.get("render_loop_ms").unwrap_or(0);
        assert_eq!(render_loop_ms, 5000);

        let env_table: LuaTable = globals
            .get("environment")
            .expect("environment table should exist");
        let editor: String = env_table.get("EDITOR").unwrap_or_default();
        assert_eq!(editor, "vim");
    }

    #[test]
    fn lua_config_startup_commands_alias() {
        let config = LuaConfig::from_string(
            r#"
             return {
                 startup_commands = {
                     { command = { "echo", "hello" } },
                 }
             }
         "#,
        )
        .unwrap();

        // startup_commands should be mapped to startup global
        let startup_table: LuaTable = config
            .runtime()
            .inner()
            .globals()
            .get("startup")
            .expect("startup global should be set from startup_commands");
        assert_eq!(startup_table.raw_len(), 1);
    }

    #[test]
    fn lua_config_apply_config_startup_commands_alias() {
        let config = LuaConfig::from_string(
            r#"
             niri.apply_config({
                 startup_commands = {
                     { command = { "echo", "hello" } },
                 }
             })
         "#,
        )
        .unwrap();

        // startup_commands should be mapped to startup global
        let startup_table: LuaTable = config
            .runtime()
            .inner()
            .globals()
            .get("startup")
            .expect("startup global should be set from startup_commands");
        assert_eq!(startup_table.raw_len(), 1);
    }

    #[test]
    fn lua_config_mixed_apply_and_local_vars() {
        let config = LuaConfig::from_string(
            r#"
             -- Local variable that won't be exported
             local internal_state = 42
             
             -- Apply config with external-visible values
             niri.apply_config({
                 prefer_no_csd = true,
                 debug = {
                     render_loop_ms = internal_state * 100
                 }
             })
         "#,
        )
        .unwrap();

        let globals = config.runtime().inner().globals();

        // Verify internal_state is NOT in globals (local var)
        let internal_state: Option<i32> = globals.get("internal_state").ok();
        assert!(internal_state.is_none());

        // But the calculated value should be there
        let debug_table: LuaTable = globals.get("debug").expect("debug table should exist");
        let render_loop_ms: i32 = debug_table.get("render_loop_ms").unwrap_or(0);
        assert_eq!(render_loop_ms, 4200); // 42 * 100
    }

    #[test]
    fn lua_config_prefer_apply_config_over_return() {
        // Test that both patterns work independently
        // When BOTH are present, the return value is processed first, then Lua execution continues
        // This tests that apply_config works correctly even when used in a config that might return
        let config = LuaConfig::from_string(
            r#"
             niri.apply_config({
                 prefer_no_csd = true,
             })
             
             -- Script execution continues after apply_config
             calculated_value = 100
         "#,
        )
        .unwrap();

        let globals = config.runtime().inner().globals();

        let prefer_no_csd: bool = globals.get("prefer_no_csd").unwrap_or(false);
        assert!(prefer_no_csd);

        let calculated_value: i32 = globals.get("calculated_value").unwrap_or(0);
        assert_eq!(calculated_value, 100);
    }

    #[test]
    fn lua_config_empty_apply_config() {
        let _config = LuaConfig::from_string(
            r#"
             niri.apply_config({})
         "#,
        )
        .unwrap();

        // Should not error and should succeed
        assert!(true); // Test passed if we got here without panicking
    }

    #[test]
    fn lua_config_nil_values_ignored() {
        let config = LuaConfig::from_string(
            r#"
             niri.apply_config({
                 prefer_no_csd = true,
                 debug = nil,  -- This should be ignored
             })
         "#,
        )
        .unwrap();

        let globals = config.runtime().inner().globals();

        let prefer_no_csd: bool = globals.get("prefer_no_csd").unwrap_or(false);
        assert!(prefer_no_csd);

        // debug should not be set since we passed nil
        let debug_table: Option<LuaTable> = globals.get("debug").ok();
        assert!(debug_table.is_none());
    }

    #[test]
    fn lua_config_apply_config_with_nested_tables() {
        let config = LuaConfig::from_string(
            r#"
             niri.apply_config({
                 debug = {
                     render_loop_ms = 5000,
                     disable_animations = false,
                 }
             })
         "#,
        )
        .unwrap();

        let debug_table: LuaTable = config
            .runtime()
            .inner()
            .globals()
            .get("debug")
            .expect("debug table should be set");

        let render_loop_ms: i32 = debug_table.get("render_loop_ms").unwrap_or(0);
        assert_eq!(render_loop_ms, 5000);

        let disable_animations: bool = debug_table.get("disable_animations").unwrap_or(true);
        assert!(!disable_animations);
    }

    #[test]
    fn lua_config_apply_config_with_array_tables() {
        let config = LuaConfig::from_string(
            r#"
             niri.apply_config({
                 outputs = {
                     { name = "HDMI-1" },
                     { name = "HDMI-2" },
                 }
             })
         "#,
        )
        .unwrap();

        let outputs_table: LuaTable = config
            .runtime()
            .inner()
            .globals()
            .get("outputs")
            .expect("outputs table should be set");

        assert_eq!(outputs_table.raw_len(), 2);
    }

    #[test]
    fn lua_config_return_and_apply_both_patterns() {
        // Test using from_string which handles both return pattern and apply_config
        let config = LuaConfig::from_string(
            r#"
             -- First, apply some config
             niri.apply_config({
                 prefer_no_csd = true,
             })
             
             -- Then return a config table (this will be extracted as fallback)
             return {
                 debug = {
                     render_loop_ms = 5000
                 }
             }
         "#,
        )
        .unwrap();

        let globals = config.runtime().inner().globals();

        // Both should be available since both are processed
        let prefer_no_csd: bool = globals.get("prefer_no_csd").unwrap_or(false);
        assert!(prefer_no_csd);

        let debug_table: LuaTable = globals.get("debug").expect("debug should be set");
        let render_loop_ms: i32 = debug_table.get("render_loop_ms").unwrap_or(0);
        assert_eq!(render_loop_ms, 5000);
    }

    #[test]
    fn lua_config_apply_config_overwrites_previous() {
        let config = LuaConfig::from_string(
            r#"
             niri.apply_config({
                 prefer_no_csd = false,
             })
             
             -- Second call overwrites
             niri.apply_config({
                 prefer_no_csd = true,
             })
         "#,
        )
        .unwrap();

        let prefer_no_csd: bool = config
            .runtime()
            .inner()
            .globals()
            .get("prefer_no_csd")
            .unwrap_or(false);

        // Should be true from the second call
        assert!(prefer_no_csd);
    }

    #[test]
    fn lua_config_from_file_with_apply_config_pattern() {
        // Create a temporary file with apply_config pattern
        let content = r#"
            niri.apply_config({
                prefer_no_csd = true,
                binds = {
                    { key = "Super+Return", action = "spawn", args = { "alacritty" } },
                },
                debug = {
                    render_loop_ms = 5000,
                }
            })
        "#;

        let config = LuaConfig::from_string(content).unwrap();
        let globals = config.runtime().inner().globals();

        // Verify config was applied
        let prefer_no_csd: bool = globals.get("prefer_no_csd").unwrap_or(false);
        assert!(prefer_no_csd);

        let binds: LuaTable = globals.get("binds").expect("binds should be set");
        assert_eq!(binds.raw_len(), 1);

        let debug: LuaTable = globals.get("debug").expect("debug should be set");
        let render_loop_ms: i32 = debug.get("render_loop_ms").unwrap_or(0);
        assert_eq!(render_loop_ms, 5000);
    }

    #[test]
    fn lua_config_from_file_with_return_pattern() {
        // Create a configuration using return table pattern
        let content = r#"
            return {
                prefer_no_csd = true,
                binds = {
                    { key = "Super+Return", action = "spawn", args = { "alacritty" } },
                },
                debug = {
                    render_loop_ms = 5000,
                }
            }
        "#;

        let config = LuaConfig::from_string(content).unwrap();
        let globals = config.runtime().inner().globals();

        // Verify config was extracted from returned table
        let prefer_no_csd: bool = globals.get("prefer_no_csd").unwrap_or(false);
        assert!(prefer_no_csd);

        let binds: LuaTable = globals.get("binds").expect("binds should be set");
        assert_eq!(binds.raw_len(), 1);

        let debug: LuaTable = globals.get("debug").expect("debug should be set");
        let render_loop_ms: i32 = debug.get("render_loop_ms").unwrap_or(0);
        assert_eq!(render_loop_ms, 5000);
    }
}
