//! Example Niri API component for Lua scripting.
//!
//! This module demonstrates how to create a Lua component that exposes
//! Niri-specific functionality to Lua scripts.

use std::rc::Rc;

use log::{debug, error, info, warn};
use mlua::prelude::*;

use crate::{fs_utils, os_utils, LuaComponent};

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

        // Register the format_value function globally for pretty-printing
        let format_value_fn: LuaFunction = lua.load(include_str!("format_value.lua")).eval()?;
        lua.globals().set("__niri_format_value", format_value_fn)?;

        // Create the utils table for logging and utility functions
        let utils = lua.create_table()?;

        // Register logging function under niri.utils (like vim.print - inspects any value)
        let log_fn = lua.create_function(|lua, args: LuaMultiValue| {
            let format_value: LuaFunction = lua.globals().get("__niri_format_value")?;
            let mut parts = Vec::new();
            for val in args.into_iter() {
                let formatted: String = format_value.call(val)?;
                parts.push(formatted);
            }
            let message = parts.join("\t");
            info!("Lua Info: {}", message);
            Ok(())
        })?;
        utils.set("log", log_fn)?;

        // Register debug print function under niri.utils (like vim.print - inspects any value)
        let debug_fn = lua.create_function(|lua, args: LuaMultiValue| {
            let format_value: LuaFunction = lua.globals().get("__niri_format_value")?;
            let mut parts = Vec::new();
            for val in args.into_iter() {
                let formatted: String = format_value.call(val)?;
                parts.push(formatted);
            }
            let message = parts.join("\t");
            debug!("Lua Debug: {}", message);
            Ok(())
        })?;
        utils.set("debug", debug_fn)?;

        // Register warning function under niri.utils (like vim.print - inspects any value)
        let warn_fn = lua.create_function(|lua, args: LuaMultiValue| {
            let format_value: LuaFunction = lua.globals().get("__niri_format_value")?;
            let mut parts = Vec::new();
            for val in args.into_iter() {
                let formatted: String = format_value.call(val)?;
                parts.push(formatted);
            }
            let message = parts.join("\t");
            warn!("Lua Warning: {}", message);
            Ok(())
        })?;
        utils.set("warn", warn_fn)?;

        // Register error function under niri.utils (like vim.print - inspects any value)
        let error_fn = lua.create_function(|lua, args: LuaMultiValue| {
            let format_value: LuaFunction = lua.globals().get("__niri_format_value")?;
            let mut parts = Vec::new();
            for val in args.into_iter() {
                let formatted: String = format_value.call(val)?;
                parts.push(formatted);
            }
            let message = parts.join("\t");
            error!("Lua Error: {}", message);
            Ok(())
        })?;
        utils.set("error", error_fn)?;

        // Register spawn function under niri.utils
        let callback_clone = callback.clone();
        let spawn_fn = lua.create_function(move |_, command: String| {
            info!("Spawning command: {}", command);
            callback_clone("spawn".to_string(), vec![command])
        })?;
        utils.set("spawn", spawn_fn)?;

        // Set niri.utils
        niri.set("utils", utils)?;

        // Register version info function
        let version_fn =
            lua.create_function(|_, ()| Ok(format!("Niri {}", env!("CARGO_PKG_VERSION"))))?;
        niri.set("version", version_fn)?;

        // Register a table with configuration helper functions
        let config = lua.create_table()?;

        // Config helper: get Niri version
        let get_version_fn =
            lua.create_function(|_, ()| Ok(env!("CARGO_PKG_VERSION").to_string()))?;
        config.set("version", get_version_fn)?;

        niri.set("config", config)?;

        // Register keymap functions
        let keymap = lua.create_table()?;

        // Keymap set function - currently just accepts and logs the keybinding
        let keymap_set_fn =
            lua.create_function(|_, (mode, key, cb): (String, String, mlua::Function)| {
                info!(
                    "Setting keymap: mode={}, key={}, callback=function",
                    mode, key
                );
                // For now, just call the callback immediately as a test
                // In a full implementation, this would register the keybinding
                let _ = cb.call::<()>(());
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
            callback_clone(
                "screenshot".to_string(),
                vec!["true".to_string(), "".to_string()],
            )
        })?;
        screenshot.set("full", screenshot_full_fn)?;

        niri.set("screenshot", screenshot)?;

        // Register a stub niri.state table for config load time
        // The real runtime API is registered later after the compositor is initialized.
        // These stubs return truthful empty data - during config load, there genuinely
        // are no windows, workspaces, or outputs yet. This follows Neovim's design where
        // vim.api always returns valid (if minimal) data rather than warnings.
        let state_stub = lua.create_table()?;

        // Returns empty array - truthful, as no windows exist during config load
        let windows_stub = lua.create_function(|lua, ()| lua.create_table())?;
        state_stub.set("windows", windows_stub)?;

        // Returns nil - truthful, as no window is focused during config load
        let focused_window_stub = lua.create_function(|_, ()| Ok(mlua::Value::Nil))?;
        state_stub.set("focused_window", focused_window_stub)?;

        // Returns empty array - truthful, as no workspaces exist during config load
        let workspaces_stub = lua.create_function(|lua, ()| lua.create_table())?;
        state_stub.set("workspaces", workspaces_stub)?;

        // Returns empty array - truthful, as no outputs are configured during config load
        let outputs_stub = lua.create_function(|lua, ()| lua.create_table())?;
        state_stub.set("outputs", outputs_stub)?;

        niri.set("state", state_stub)?;

        // Register apply_config function
        // This function takes a config table and applies it to globals
        // allowing scripts to do: niri.apply_config({ binds = {...}, input = {...} })
        let apply_config_fn = lua.create_function(|lua, config_table: LuaTable| {
            info!("Applying configuration from Lua table");
            let globals = lua.globals();

            // List of config fields to extract and set as globals
            let field_names = [
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
            ];

            for field_name in &field_names {
                if let Ok(value) = config_table.get::<LuaValue>(*field_name) {
                    if value != LuaValue::Nil {
                        debug!("Applying config field: {}", field_name);
                        globals.set(*field_name, value)?;
                    }
                }
            }

            // Also check for startup_commands and map it to startup
            if let Ok(value) = config_table.get::<LuaValue>("startup_commands") {
                if value != LuaValue::Nil {
                    debug!("Applying startup_commands (mapping to startup)");
                    globals.set("startup", value)?;
                }
            }

            info!("✓ Configuration applied successfully");
            Ok(())
        })?;
        niri.set("apply_config", apply_config_fn)?;

        // Register nice_print function as niri.print
        let nice_print_code = include_str!("nice_print.lua");
        let nice_print_fn: LuaFunction = lua.load(nice_print_code).eval()?;
        niri.set("print", nice_print_fn)?;

        // Register the niri table globally
        globals.set("niri", niri.clone())?;

        // Register OS and filesystem utilities
        os_utils::register(lua, &niri)?;
        fs_utils::register(lua, &niri)?;

        info!("Registered Niri API component to Lua");

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use log::info;

    use super::{Lua, LuaComponent, LuaFunction, LuaStdLib, LuaTable, NiriApi};

    #[test]
    fn niri_api_registration() {
        let lua = Lua::new();
        // Load only safe standard libraries (exclude debug)
        lua.load_std_libs(LuaStdLib::ALL_SAFE).unwrap();

        let result = NiriApi::register_to_lua(&lua, |action, args| {
            info!("Test action: {} with args {:?}", action, args);
            Ok(())
        });
        assert!(result.is_ok());

        // Verify niri table exists with utils namespace
        let niri: LuaTable = lua.globals().get("niri").unwrap();
        let utils: LuaTable = niri.get("utils").unwrap();
        let log_fn: LuaFunction = utils.get("log").unwrap();
        let debug_fn: LuaFunction = utils.get("debug").unwrap();
        // Just verify they're functions by successfully getting them
        drop(log_fn);
        drop(debug_fn);
    }

    #[test]
    fn niri_api_utils_namespace() {
        let lua = Lua::new();
        lua.load_std_libs(LuaStdLib::ALL_SAFE).unwrap();

        let result = NiriApi::register_to_lua(&lua, |action, args| {
            info!("Test action: {} with args {:?}", action, args);
            Ok(())
        });
        assert!(result.is_ok());

        // Verify niri.utils table exists with all functions
        let niri: LuaTable = lua.globals().get("niri").unwrap();
        let utils: LuaTable = niri.get("utils").unwrap();

        // Verify all utility functions exist
        let _: LuaFunction = utils.get("log").unwrap();
        let _: LuaFunction = utils.get("debug").unwrap();
        let _: LuaFunction = utils.get("warn").unwrap();
        let _: LuaFunction = utils.get("error").unwrap();
        let _: LuaFunction = utils.get("spawn").unwrap();
    }

    #[test]
    fn niri_api_logging() {
        let lua = Lua::new();
        // Load only safe standard libraries (exclude debug)
        lua.load_std_libs(LuaStdLib::ALL_SAFE).unwrap();
        NiriApi::register_to_lua(&lua, |action, args| {
            info!("Test action: {} with args {:?}", action, args);
            Ok(())
        })
        .unwrap();

        let result = lua
            .load(
                r#"
            niri.utils.log("Test message")
            niri.utils.debug("Debug message")
            niri.utils.warn("Warning message")
            niri.utils.error("Error message")
        "#,
            )
            .exec();

        assert!(result.is_ok());
    }

    #[test]
    fn niri_api_utils_logging() {
        let lua = Lua::new();
        lua.load_std_libs(LuaStdLib::ALL_SAFE).unwrap();
        NiriApi::register_to_lua(&lua, |action, args| {
            info!("Test action: {} with args {:?}", action, args);
            Ok(())
        })
        .unwrap();

        // Test new niri.utils.* namespace
        let result = lua
            .load(
                r#"
            niri.utils.log("Test message via utils")
            niri.utils.debug("Debug message via utils")
            niri.utils.warn("Warning message via utils")
            niri.utils.error("Error message via utils")
        "#,
            )
            .exec();

        assert!(result.is_ok());
    }

    #[test]
    fn niri_api_state_stub() {
        let lua = Lua::new();
        lua.load_std_libs(LuaStdLib::ALL_SAFE).unwrap();
        NiriApi::register_to_lua(&lua, |action, args| {
            info!("Test action: {} with args {:?}", action, args);
            Ok(())
        })
        .unwrap();

        // Verify niri.state stub exists and returns truthful empty data during config load.
        // This follows Neovim's design: vim.api always returns valid data, even if minimal.
        // During config load, there genuinely are no windows/workspaces yet, so empty is correct.
        let niri: LuaTable = lua.globals().get("niri").unwrap();
        let state: LuaTable = niri.get("state").unwrap();

        // Verify all stub functions exist
        let _: LuaFunction = state.get("windows").unwrap();
        let _: LuaFunction = state.get("focused_window").unwrap();
        let _: LuaFunction = state.get("workspaces").unwrap();
        let _: LuaFunction = state.get("outputs").unwrap();

        // Test that calling the stubs doesn't error and returns appropriate values
        let result = lua
            .load(
                r#"
            local windows = niri.state.windows()
            local focused = niri.state.focused_window()
            local workspaces = niri.state.workspaces()
            local outputs = niri.state.outputs()

            -- Stubs should return empty tables or nil
            assert(type(windows) == "table", "windows should be a table")
            assert(focused == nil, "focused_window should be nil")
            assert(type(workspaces) == "table", "workspaces should be a table")
            assert(type(outputs) == "table", "outputs should be a table")

            -- Empty tables should have length 0
            assert(#windows == 0, "windows should be empty")
            assert(#workspaces == 0, "workspaces should be empty")
            assert(#outputs == 0, "outputs should be empty")
        "#,
            )
            .exec();

        assert!(
            result.is_ok(),
            "State stub calls should succeed: {:?}",
            result
        );
    }

    #[test]
    fn spawn_triggers_callback() {
        use std::cell::RefCell;
        use std::rc::Rc;

        let lua = Lua::new();
        lua.load_std_libs(LuaStdLib::ALL_SAFE).unwrap();

        let spawned = Rc::new(RefCell::new(None));
        let spawned_clone = spawned.clone();

        NiriApi::register_to_lua(&lua, move |action, args| {
            *spawned_clone.borrow_mut() = Some((action, args));
            Ok(())
        })
        .unwrap();

        lua.load(r#"niri.utils.spawn("alacritty")"#).exec().unwrap();

        let result = spawned.borrow();
        assert!(result.is_some());
        let (action, args) = result.as_ref().unwrap();
        assert_eq!(action, "spawn");
        assert_eq!(args, &vec!["alacritty".to_string()]);
    }

    #[test]
    fn logging_multiple_args() {
        let lua = Lua::new();
        lua.load_std_libs(LuaStdLib::ALL_SAFE).unwrap();
        NiriApi::register_to_lua(&lua, |_, _| Ok(())).unwrap();

        // Test that log functions accept multiple arguments
        let result = lua
            .load(
                r#"
            niri.utils.log("arg1", 42, {key = "value"}, nil, true)
            niri.utils.debug(1, 2, 3)
            niri.utils.warn("warning", "with", "multiple", "parts")
        "#,
            )
            .exec();

        assert!(result.is_ok());
    }

    #[test]
    fn logging_complex_values() {
        let lua = Lua::new();
        lua.load_std_libs(LuaStdLib::ALL_SAFE).unwrap();
        NiriApi::register_to_lua(&lua, |_, _| Ok(())).unwrap();

        // Test logging of complex nested structures
        let result = lua
            .load(
                r#"
            local nested = {
                level1 = {
                    level2 = {
                        value = "deep"
                    }
                },
                array = {1, 2, 3}
            }
            niri.utils.log(nested)
            niri.utils.debug(function() end)  -- functions should be handled
        "#,
            )
            .exec();

        assert!(result.is_ok());
    }
}
