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
        let runtime = LuaRuntime::new()
            .map_err(|e| anyhow::anyhow!("Failed to create Lua runtime from file: {}", e))?;

        info!("Loading Lua config from {}", path_ref.display());

        // Register the Niri API component
        runtime
            .register_component(|action, args| {
                info!("Lua action: {} with args {:?}", action, args);
                Ok(())
            })
            .map_err(|e| anyhow::anyhow!("Failed to register Niri API: {}", e))?;

        debug!("Niri API registered successfully");

        // Load the configuration file
        // Capture the return value in case the script returns a configuration table
        let return_val = runtime
            .load_file(&path_ref)
            .map_err(|e| anyhow::anyhow!("Failed to load Lua config file: {}", e))?;

        // If the script returns a table, extract its fields and set them as globals
        if let LuaValue::Table(config_table) = return_val {
            debug!("Lua file returned a table, extracting configuration");

            // Extract fields from returned table and set as globals
            // This allows scripts to use local variables and return a config table
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
        let runtime = LuaRuntime::new()
            .map_err(|e| anyhow::anyhow!("Failed to create Lua runtime from string: {}", e))?;

        // Register the Niri API component
        runtime
            .register_component(|action, args| {
                info!("Lua action: {} with args {:?}", action, args);
                Ok(())
            })
            .map_err(|e| anyhow::anyhow!("Failed to register Niri API: {}", e))?;

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
             niri.log("Test log message")
             version_str = niri.config.version()
         "#,
        )
        .unwrap();

        // Verify it executed without error
        let version_str: String = config
            .runtime()
            .inner()
            .globals()
            .get("version_str")
            .unwrap();
        assert!(!version_str.is_empty());
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
}
