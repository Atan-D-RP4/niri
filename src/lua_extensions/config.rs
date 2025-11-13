//! Lua configuration support for Niri.
//!
//! This module handles loading and executing Lua configuration files.
//! It integrates with Niri's config system to allow Lua-based scripting.

use anyhow::Result;
use std::path::Path;
use crate::lua_extensions::LuaRuntime;

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
        let runtime = LuaRuntime::new()
            .map_err(|e| anyhow::anyhow!("Failed to create Lua runtime: {}", e))?;

        // Register the Niri API component
        runtime
            .register_component(|action, args| {
                info!("Lua action: {} with args {:?}", action, args);
                Ok(())
            })
            .map_err(|e| anyhow::anyhow!("Failed to register Niri API: {}", e))?;

        // Load the configuration file
        // The return value is discarded - we focus on side effects and global state
        let _ = runtime
            .load_file(&path)
            .map_err(|e| anyhow::anyhow!("Failed to load Lua config file: {}", e))?;

        info!(
            "Loaded Lua configuration from {}",
            path.as_ref().display()
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
            .map_err(|e| anyhow::anyhow!("Failed to create Lua runtime: {}", e))?;

        // Register the Niri API component
        runtime
            .register_component(|action, args| {
                info!("Lua action: {} with args {:?}", action, args);
                Ok(())
            })
            .map_err(|e| anyhow::anyhow!("Failed to register Niri API: {}", e))?;

        // Load and execute the code
        let _ = runtime
            .load_string(code)
            .map_err(|e| anyhow::anyhow!("Failed to execute Lua code: {}", e))?;

        Ok(Self { runtime })
    }

    /// Get a reference to the underlying Lua runtime.
    ///
    /// This allows advanced users to access the full mlua API.
    pub fn runtime(&self) -> &LuaRuntime {
        &self.runtime
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lua_config_from_string() {
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
    fn test_lua_niri_api() {
        let config = LuaConfig::from_string(
            r#"
            niri.log("Test log message")
            version_str = niri.config.version()
        "#,
        ).unwrap();

        // Verify it executed without error
        assert!(config.runtime().inner().globals().get::<_, String>("version_str").is_ok());
    }
}
