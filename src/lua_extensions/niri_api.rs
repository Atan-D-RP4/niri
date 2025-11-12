//! Example Niri API component for Lua scripting.
//!
//! This module demonstrates how to create a Lua component that exposes
//! Niri-specific functionality to Lua scripts.

use mlua::prelude::*;
use crate::lua_extensions::LuaComponent;
use std::rc::Rc;

/// Niri logging and utility functions for Lua.
///
/// This component provides basic logging and utility functions that Lua scripts
/// can use to interact with Niri.
pub struct NiriApi;

impl LuaComponent for NiriApi {
    fn register_to_lua<F>(lua: &Lua, action_callback: F) -> LuaResult<()>
    where
        F: Fn(String, Vec<String>) -> LuaResult<()> + 'static,
    {
        let callback = Rc::new(action_callback);
        let globals = lua.globals();

        // Create the niri table
        let niri = lua.create_table()?;

        // Register logging function
        let log_fn = lua.create_function(|_, message: String| {
            info!("Lua: {}", message);
            Ok(())
        })?;
        niri.set("log", log_fn)?;

        // Register debug print function
        let debug_fn = lua.create_function(|_, message: String| {
            debug!("Lua Debug: {}", message);
            Ok(())
        })?;
        niri.set("debug", debug_fn)?;

        // Register warning function
        let warn_fn = lua.create_function(|_, message: String| {
            warn!("Lua Warning: {}", message);
            Ok(())
        })?;
        niri.set("warn", warn_fn)?;

        // Register error function
        let error_fn = lua.create_function(|_, message: String| {
            error!("Lua Error: {}", message);
            Ok(())
        })?;
        niri.set("error", error_fn)?;

        // Register version info function
        let version_fn = lua.create_function(|_, ()| {
            Ok(format!("Niri {}", crate::utils::version()))
        })?;
        niri.set("version", version_fn)?;

        // Register a table with configuration helper functions
        let config = lua.create_table()?;

        // Config helper: get Niri version
        let get_version_fn = lua.create_function(|_, ()| {
            Ok(crate::utils::version().to_string())
        })?;
        config.set("version", get_version_fn)?;

        niri.set("config", config)?;

        // Register keymap functions
        let keymap = lua.create_table()?;

        // Keymap set function - currently just accepts and logs the keybinding
        let keymap_set_fn = lua.create_function(|_, (mode, key, cb): (String, String, mlua::Function)| {
            info!("Setting keymap: mode={}, key={}, callback=function", mode, key);
            // For now, just call the callback immediately as a test
            // In a full implementation, this would register the keybinding
            let _ = cb.call::<_, ()>(());
            Ok(())
        })?;
        keymap.set("set", keymap_set_fn)?;

        niri.set("keymap", keymap)?;

        // Register window functions
        let window = lua.create_table()?;

        // Window close function
        let callback_clone = callback.clone();
        let window_close_fn = lua.create_function(move |_, ()| {
            info!("Closing window");
            callback_clone("close_window".to_string(), vec![])
        })?;
        window.set("close", window_close_fn)?;

        niri.set("window", window)?;

        // Register overview functions
        let overview = lua.create_table()?;

        // Overview toggle function
        let callback_clone = callback.clone();
        let overview_toggle_fn = lua.create_function(move |_, ()| {
            info!("Toggling overview");
            callback_clone("toggle_overview".to_string(), vec![])
        })?;
        overview.set("toggle", overview_toggle_fn)?;

        niri.set("overview", overview)?;

        // Register screenshot functions
        let screenshot = lua.create_table()?;

        // Screenshot full function
        let callback_clone = callback.clone();
        let screenshot_full_fn = lua.create_function(move |_, ()| {
            info!("Taking full screenshot");
            callback_clone("screenshot".to_string(), vec!["true".to_string(), "".to_string()])
        })?;
        screenshot.set("full", screenshot_full_fn)?;

        niri.set("screenshot", screenshot)?;

        // Register spawn function
        let callback_clone = callback.clone();
        let spawn_fn = lua.create_function(move |_, command: String| {
            info!("Spawning command: {}", command);
            callback_clone("spawn".to_string(), vec![command])
        })?;
        niri.set("spawn", spawn_fn)?;

        // Register the niri table globally
        globals.set("niri", niri)?;

        // Also register common utility functions at global level
        let pprint_fn = lua.create_function(|_, value: LuaValue| {
            let json_str = match &value {
                LuaValue::Nil => "nil".to_string(),
                LuaValue::Boolean(b) => b.to_string(),
                LuaValue::Integer(i) => i.to_string(),
                LuaValue::Number(n) => n.to_string(),
                LuaValue::String(s) => format!("\"{}\"", s.to_str().unwrap_or("<invalid utf8>")),
                _ => format!("{:?}", value),
            };
            println!("{}", json_str);
            Ok(())
        })?;
        globals.set("pprint", pprint_fn)?;

        info!("Registered Niri API component to Lua");

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_niri_api_registration() {
        let lua = Lua::new();
        lua.load_from_std_lib(LuaStdLib::ALL).unwrap();

        let result = NiriApi::register_to_lua(&lua, |action, args| {
            info!("Test action: {} with args {:?}", action, args);
            Ok(())
        });
        assert!(result.is_ok());

        // Verify niri table exists
        let niri: LuaTable = lua.globals().get("niri").unwrap();
        assert!(niri.get::<_, LuaFunction>("log").is_ok());
        assert!(niri.get::<_, LuaFunction>("debug").is_ok());
    }

    #[test]
    fn test_niri_api_logging() {
        let lua = Lua::new();
        lua.load_from_std_lib(LuaStdLib::ALL).unwrap();
        NiriApi::register_to_lua(&lua, |action, args| {
            info!("Test action: {} with args {:?}", action, args);
            Ok(())
        }).unwrap();

        let result = lua.load(r#"
            niri.log("Test message")
            niri.debug("Debug message")
            niri.warn("Warning message")
            niri.error("Error message")
        "#).exec();

        assert!(result.is_ok());
    }
}
