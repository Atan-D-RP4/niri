//! Configuration API module for exposing Niri settings to Lua.
//!
//! This module provides the `niri.config` API that allows Lua scripts to read and configure Niri settings.
//! Currently this is a placeholder for future implementation.

use mlua::prelude::*;

/// Main configuration API handler
pub struct ConfigApi;

impl ConfigApi {
    /// Register the configuration API to Lua
    pub fn register_to_lua(lua: &Lua) -> LuaResult<()> {
        let globals = lua.globals();

        // Get or create the niri table
        let niri_table: LuaTable = globals
            .get("niri")
            .unwrap_or_else(|_| lua.create_table().unwrap());

        // Create the config table
        let config_table = lua.create_table()?;

        // TODO: Register configuration subsystems:
        // - animations
        // - input (keyboard, mouse, touchpad)
        // - layout
        // - gestures
        // - appearance
        // - outputs

        // Set niri.config
        niri_table.set("config", config_table)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_api_registration() {
        let lua = Lua::new();
        // Create niri table first
        let globals = lua.globals();
        let niri_table = lua.create_table().unwrap();
        globals.set("niri", niri_table).unwrap();

        // Register should not fail
        let result = ConfigApi::register_to_lua(&lua);
        assert!(result.is_ok());

        // Verify config table was created
        let config: LuaTable = globals.get("niri").unwrap();
        let config_table: LuaTable = config.get("config").unwrap();
        assert!(!config_table.is_empty());
    }
}
