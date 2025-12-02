//! Lua runtime initialization and management.
//!
//! This module handles creating and managing the Lua runtime with LuaJIT.
//! It provides utilities for loading scripts and managing the Lua environment.

use std::path::Path;
use std::sync::Arc;

use mlua::prelude::*;
use niri_config::Config;

use crate::config_api::ConfigApi;
use crate::config_proxy::{create_shared_pending_changes, register_config_proxy_to_lua, SharedPendingChanges};
use crate::event_handlers::EventHandlers;
use crate::event_system::EventSystem;
use crate::events_proxy::register_events_proxy;
use crate::action_proxy::{register_action_proxy, ActionCallback};
use crate::{LuaComponent, NiriApi};

/// Manages a Lua runtime for Niri.
///
/// This struct encapsulates the Lua runtime and provides methods for
/// executing scripts and registering components.
pub struct LuaRuntime {
    lua: Lua,
    /// Event system for emitting Lua events from the compositor
    pub event_system: Option<EventSystem>,
    /// Pending configuration changes (for the new v2 API)
    pub pending_config: Option<SharedPendingChanges>,
}

impl LuaRuntime {
    /// Create a new Lua runtime.
    ///
    /// # Errors
    ///
    /// Returns an error if the Lua runtime cannot be created.
    pub fn new() -> LuaResult<Self> {
        let lua = Lua::new();

        // Set up standard library with appropriate restrictions
        lua.load_std_libs(LuaStdLib::ALL_SAFE)?;

        Ok(Self {
            lua,
            event_system: None,
            pending_config: None,
        })
    }

    /// Register a Lua component, adding its functions to the runtime.
    ///
    /// # Errors
    ///
    /// Returns an error if component registration fails.
    pub fn register_component<F>(&self, action_callback: F) -> LuaResult<()>
    where
        F: Fn(String, Vec<String>) -> LuaResult<()> + 'static,
    {
        NiriApi::register_to_lua(&self.lua, action_callback)
    }

    /// Register the configuration API to the runtime.
    ///
    /// This provides access to configuration settings through the niri.config table.
    ///
    /// # Arguments
    ///
    /// * `config` - The current Niri configuration to expose to Lua
    ///
    /// # Errors
    ///
    /// Returns an error if configuration API registration fails.
    pub fn register_config_api(&self, config: &Config) -> LuaResult<()> {
        ConfigApi::register_to_lua(&self.lua, config)
    }

    /// Register the reactive configuration proxy API to the runtime.
    ///
    /// This provides a reactive configuration system through `niri.config` that allows
    /// reading current values and staging changes via proxy tables. Changes are accumulated
    /// until `niri.config:apply()` is called, or applied automatically if auto-apply mode
    /// is enabled via `niri.config:auto_apply(true)`.
    ///
    /// # Arguments
    ///
    /// * `config` - The current Niri configuration to initialize proxy values from
    ///
    /// # Returns
    ///
    /// Returns a shared handle to the pending changes that can be used by the compositor
    /// to apply configuration updates.
    ///
    /// # Errors
    ///
    /// Returns an error if configuration proxy API registration fails.
    pub fn register_config_proxy_api(&mut self, config: &Config) -> LuaResult<SharedPendingChanges> {
        let pending = create_shared_pending_changes();
        register_config_proxy_to_lua(&self.lua, pending.clone(), config)?;
        self.pending_config = Some(pending.clone());
        Ok(pending)
    }

    /// Initialize an empty config proxy for initial script loading.
    ///
    /// This creates a config proxy with empty collections that can be populated
    /// by the Lua script during initial loading. The proxy will be updated with
    /// real config values later when `register_config_proxy_api` is called.
    ///
    /// Spawn commands issued via `niri.action:spawn()` during config loading
    /// are captured and added to `spawn_at_startup` in the pending config changes.
    ///
    /// # Errors
    ///
    /// Returns an error if configuration proxy initialization fails.
    pub fn init_empty_config_proxy(&mut self) -> LuaResult<SharedPendingChanges> {
        let pending = create_shared_pending_changes();
        // Use a default config to initialize empty collections
        let default_config = Config::default();
        register_config_proxy_to_lua(&self.lua, pending.clone(), &default_config)?;
        self.pending_config = Some(pending.clone());

        // Also register the action proxy with a callback that captures spawn commands
        // Spawn actions called during config loading are converted to spawn_at_startup
        let pending_clone = pending.clone();
        let capture_callback: ActionCallback = std::sync::Arc::new(move |action| {
            match &action {
                niri_ipc::Action::Spawn { command } => {
                    log::debug!("Config load: capturing spawn {:?} as spawn_at_startup", command);
                    // Add to pending config changes as spawn_at_startup
                    let spawn_json = serde_json::json!(command);
                    let mut pending = pending_clone.lock();
                    pending.collection_additions
                        .entry("spawn_at_startup".to_string())
                        .or_default()
                        .push(spawn_json);
                }
                _ => {
                    log::debug!("Config load: action {:?} (deferred/ignored)", action);
                }
            }
            Ok(())
        });
        register_action_proxy(&self.lua, capture_callback)?;

        Ok(pending)
    }

    /// Register the runtime API to the runtime.
    ///
    /// This provides access to live compositor state through the niri.runtime table.
    /// The runtime API allows querying windows, workspaces, outputs, and other dynamic state.
    ///
    /// # Type Parameters
    ///
    /// * `S` - The compositor state type that implements `CompositorState`
    ///
    /// # Arguments
    ///
    /// * `api` - The RuntimeApi instance connected to the compositor's event loop
    ///
    /// # Errors
    ///
    /// Returns an error if runtime API registration fails.
    pub fn register_runtime_api<S>(&self, api: crate::RuntimeApi<S>) -> LuaResult<()>
    where
        S: crate::CompositorState + 'static,
    {
        crate::register_runtime_api(&self.lua, api)
    }

    /// Initialize the event system for this runtime.
    ///
    /// This must be called once during runtime initialization to enable event handling.
    /// It registers the events proxy API (`niri.events:on()`, `niri.events:once()`, etc.)
    /// and creates the internal event system for emitting events from the compositor.
    ///
    /// # Errors
    ///
    /// Returns an error if event system initialization fails.
    pub fn init_event_system(&mut self) -> LuaResult<()> {
        let handlers = Arc::new(parking_lot::Mutex::new(EventHandlers::new()));
        
        // Register events proxy API (niri.events:on, niri.events:once, etc.)
        register_events_proxy(&self.lua, handlers.clone())?;
        
        self.event_system = Some(EventSystem::new(handlers));
        Ok(())
    }

    /// Register the action proxy API to the runtime.
    ///
    /// This provides access to compositor actions through the `niri.action` namespace.
    /// Actions are executed via the provided callback, which typically sends them
    /// to the compositor for processing.
    ///
    /// # Arguments
    ///
    /// * `callback` - Callback function that executes actions in the compositor
    ///
    /// # Example
    ///
    /// ```ignore
    /// // In the compositor, provide a callback that handles actions
    /// runtime.register_action_proxy(Arc::new(|action| {
    ///     // Handle the action (e.g., send via IPC or execute directly)
    ///     Ok(())
    /// }))?;
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if action proxy registration fails.
    pub fn register_action_proxy(&self, callback: ActionCallback) -> LuaResult<()> {
        register_action_proxy(&self.lua, callback)
    }

    /// Load and execute a Lua script from a file.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read or the script fails to execute.
    pub fn load_file<P: AsRef<Path>>(&self, path: P) -> LuaResult<LuaValue> {
        let code = std::fs::read_to_string(path)
            .map_err(|e| LuaError::external(format!("Failed to read Lua file: {}", e)))?;

        self.lua.load(&code).eval()
    }

    /// Load and execute a Lua script from a string.
    ///
    /// # Errors
    ///
    /// Returns an error if the script fails to parse or execute.
    pub fn load_string(&self, code: &str) -> LuaResult<LuaValue> {
        self.lua.load(code).eval()
    }

    /// Execute Lua code and capture output (for REPL/CLI usage).
    ///
    /// Captures print() output and returns the result.
    /// Returns a tuple of (output, success).
    pub fn execute_string(&self, code: &str) -> (String, bool) {
        // Capture print output
        let original_print = self.lua.globals().get::<LuaFunction>("print");
        let mut output = Vec::new();
        let output_ptr = &mut output as *mut Vec<String>;

        let print_fn =
            self.lua
                .create_function(move |_, args: mlua::MultiValue| -> LuaResult<()> {
                    let output_vec = unsafe { &mut *output_ptr };
                    for v in args.iter() {
                        let s = match v {
                            LuaValue::String(s) => s.to_string_lossy().to_string(),
                            LuaValue::Integer(i) => i.to_string(),
                            LuaValue::Number(n) => {
                                // Format numbers cleanly without debug info
                                if n.is_finite() {
                                    if n.fract() == 0.0 && n.abs() < 1e15 {
                                        format!("{:.0}", n)
                                    } else {
                                        n.to_string()
                                    }
                                } else if n.is_nan() {
                                    "nan".to_string()
                                } else if n.is_infinite() {
                                    if n.is_sign_positive() {
                                        "inf".to_string()
                                    } else {
                                        "-inf".to_string()
                                    }
                                } else {
                                    n.to_string()
                                }
                            }
                            LuaValue::Boolean(b) => b.to_string(),
                            LuaValue::Nil => "nil".to_string(),
                            _ => format!("{:?}", v),
                        };
                        output_vec.push(s);
                    }
                    Ok(())
                });

        if let Ok(pf) = print_fn {
            let _ = self.lua.globals().set("print", pf);
        }

        let result = self.lua.load(code).eval::<LuaValue>();

        // Restore original print
        if let Ok(orig) = original_print {
            let _ = self.lua.globals().set("print", orig);
        }

        let (success, message) = match result {
            Ok(val) => {
                // Format simple return values; tables and complex types are nil
                let val_str = match val {
                    LuaValue::Nil => String::new(),
                    LuaValue::String(s) => s.to_string_lossy().to_string(),
                    LuaValue::Integer(i) => i.to_string(),
                    LuaValue::Number(n) => {
                        if n.is_finite() {
                            if n.fract() == 0.0 && n.abs() < 1e15 {
                                format!("{:.0}", n)
                            } else {
                                n.to_string()
                            }
                        } else if n.is_nan() {
                            "nan".to_string()
                        } else if n.is_infinite() {
                            if n.is_sign_positive() {
                                "inf".to_string()
                            } else {
                                "-inf".to_string()
                            }
                        } else {
                            n.to_string()
                        }
                    }
                    LuaValue::Boolean(b) => b.to_string(),
                    _ => String::new(),
                };
                (true, val_str)
            }
            Err(e) => (false, format!("Error: {}", e)),
        };

        let full_output = if output.is_empty() {
            message
        } else if message.is_empty() {
            output.join("\n")
        } else {
            format!("{}\n{}", output.join("\n"), message)
        };

        (full_output, success)
    }

    /// Execute a Lua function that takes no arguments and returns no value.
    ///
    /// # Errors
    ///
    /// Returns an error if the function doesn't exist or execution fails.
    pub fn call_function_void(&self, name: &str) -> LuaResult<()> {
        let func: LuaFunction = self.lua.globals().get(name)?;
        func.call::<()>(())?;
        Ok(())
    }
    /// Get a reference to the underlying Lua runtime for advanced use cases.
    ///
    /// This allows direct access to the mlua::Lua instance.
    pub fn inner(&self) -> &Lua {
        &self.lua
    }
}

impl Default for LuaRuntime {
    fn default() -> Self {
        Self::new().expect("Failed to create default Lua runtime")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // LuaRuntime::new tests
    // ========================================================================

    #[test]
    fn new_runtime() {
        let rt = LuaRuntime::new();
        assert!(rt.is_ok());
    }

    // ========================================================================
    // LuaRuntime::load_file tests
    // ========================================================================

    #[test]
    fn load_file_not_found() {
        let rt = LuaRuntime::new().unwrap();
        let result = rt.load_file("/nonexistent/path.lua");
        assert!(result.is_err());
    }

    // ========================================================================
    // LuaRuntime::load_string tests
    // ========================================================================

    #[test]
    fn load_string_valid_code() {
        let rt = LuaRuntime::new().unwrap();
        let result = rt.load_string("local x = 1 + 1");
        assert!(result.is_ok());
    }

    #[test]
    fn load_string_syntax_error() {
        let rt = LuaRuntime::new().unwrap();
        let result = rt.load_string("local x = ");
        assert!(result.is_err());
    }

    // ========================================================================
    // LuaRuntime::execute_string tests
    // ========================================================================

    #[test]
    fn execute_string_valid_code() {
        let rt = LuaRuntime::new().unwrap();
        let (output, success) = rt.execute_string("return 1 + 1");
        assert!(success);
        assert!(output.contains("2"));
    }

    #[test]
    fn execute_string_syntax_error() {
        let rt = LuaRuntime::new().unwrap();
        let (output, success) = rt.execute_string("local x = ");
        assert!(!success);
        assert!(!output.is_empty());
    }

    // ========================================================================
    // LuaRuntime::call_function_void tests
    // ========================================================================

    #[test]
    fn call_function_void_exists() {
        let rt = LuaRuntime::new().unwrap();
        rt.load_string("test_func = function() return end").unwrap();
        let result = rt.call_function_void("test_func");
        assert!(result.is_ok());
    }

    #[test]
    fn call_function_void_not_exists() {
        let rt = LuaRuntime::new().unwrap();
        let result = rt.call_function_void("nonexistent_func");
        assert!(result.is_err());
    }

    // ========================================================================
    // Config Proxy API tests
    // ========================================================================

    #[test]
    fn init_empty_config_proxy_creates_shared_changes() {
        let mut rt = LuaRuntime::new().unwrap();
        let shared = rt.init_empty_config_proxy();
        assert!(shared.is_ok());
    }

    #[test]
    fn config_proxy_captures_layout_changes() {
        let mut rt = LuaRuntime::new().unwrap();
        let shared = rt.init_empty_config_proxy().unwrap();
        
        // Set a layout value through the proxy
        rt.load_string("niri.config.layout.gaps = 20").unwrap();
        
        // Check that the change was captured in scalar_changes
        let changes = shared.lock();
        let layout_gaps = changes.scalar_changes.get("layout.gaps");
        assert!(layout_gaps.is_some());
    }

    #[test]
    fn config_proxy_captures_nested_values() {
        let mut rt = LuaRuntime::new().unwrap();
        let shared = rt.init_empty_config_proxy().unwrap();
        
        // Set a deeply nested value
        rt.load_string("niri.config.layout.border.active.color = '#ff0000'").unwrap();
        
        let changes = shared.lock();
        let border_color = changes.scalar_changes.get("layout.border.active.color");
        assert!(border_color.is_some());
    }

    #[test]
    fn config_proxy_captures_bind_additions() {
        let mut rt = LuaRuntime::new().unwrap();
        let shared = rt.init_empty_config_proxy().unwrap();
        
        // Add a keybind through the collection API
        rt.load_string(r#"
            niri.config.binds:add({
                key = 'Mod+Return',
                action = 'spawn',
                args = { 'alacritty' }
            })
        "#).unwrap();
        
        let changes = shared.lock();
        // Collection additions are keyed by collection name
        let binds = changes.collection_additions.get("binds");
        assert!(binds.is_some());
        assert!(!binds.unwrap().is_empty());
    }

    #[test]
    fn config_proxy_captures_spawn_at_startup() {
        let mut rt = LuaRuntime::new().unwrap();
        let shared = rt.init_empty_config_proxy().unwrap();
        
        // Add a spawn-at-startup command
        rt.load_string(r#"
            niri.config.spawn_at_startup:add({
                command = { 'waybar' }
            })
        "#).unwrap();
        
        let changes = shared.lock();
        // Collection additions are keyed by collection name
        let spawns = changes.collection_additions.get("spawn_at_startup");
        assert!(spawns.is_some());
        assert!(!spawns.unwrap().is_empty());
    }
}
