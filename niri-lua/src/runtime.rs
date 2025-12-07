//! Lua runtime initialization and management.
//!
//! This module handles creating and managing the Lua runtime with Luau.
//! It provides utilities for loading scripts and managing the Lua environment.
//!
//! # Timeout Protection with Luau's set_interrupt
//!
//! Unlike LuaJIT (where debug hooks don't fire with JIT active), Luau provides
//! a reliable `set_interrupt` callback that fires periodically during execution,
//! even in optimized code. This allows us to implement wall-clock timeout
//! protection without sacrificing performance.
//!
//! The timeout mechanism:
//! - Uses `Instant::now()` for wall-clock time measurement
//! - Configurable via `ExecutionLimits` with sensible defaults
//! - Fires periodically (Luau guarantees this at function calls and loop iterations)
//! - Returns `VmState::Yield` to cleanly terminate runaway scripts
//!
//! # Async Primitives
//!
//! In addition to timeout protection, we provide:
//! - `niri.schedule(fn)` - defer work to next event loop iteration
//! - Worker threads (Phase 3) - offload heavy computation
//! - `niri.loop` timers (Phase 4) - time-based scheduling

use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::path::Path;
use std::rc::Rc;
use std::sync::Arc;
use std::time::{Duration, Instant};

use mlua::prelude::*;
use niri_config::Config;

use crate::action_proxy::{register_action_proxy, ActionCallback};
use crate::config_api::ConfigApi;
use crate::config_proxy::{
    create_shared_pending_changes, register_config_proxy_to_lua, update_config_proxy_values,
    SharedPendingChanges,
};
use crate::event_handlers::EventHandlers;
use crate::event_system::EventSystem;
use crate::events_proxy::register_events_proxy;
use crate::{LuaComponent, NiriApi};

/// Maximum callbacks to execute per flush cycle.
/// This bounds latency while allowing some callback chaining.
const MAX_CALLBACKS_PER_FLUSH: usize = 16;

/// Maximum queue size to prevent unbounded growth.
/// If exceeded, `niri.schedule()` will return an error.
const MAX_QUEUE_SIZE: usize = 1000;

/// Configuration for Lua execution timeouts.
///
/// These limits prevent runaway scripts from freezing the compositor.
/// When a script exceeds the timeout, it is terminated with an error.
#[derive(Debug, Clone)]
pub struct ExecutionLimits {
    /// Maximum wall-clock time per Lua execution (Duration::ZERO = unlimited).
    /// Default is 1 second.
    pub timeout: Duration,
}

impl Default for ExecutionLimits {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(1),
        }
    }
}

impl ExecutionLimits {
    /// Create limits with no timeout (unlimited execution).
    ///
    /// **Warning**: Only use this for trusted code. Unlimited execution
    /// allows scripts to freeze the compositor indefinitely.
    pub fn unlimited() -> Self {
        Self {
            timeout: Duration::ZERO,
        }
    }

    /// Create limits with a custom timeout.
    pub fn with_timeout(timeout: Duration) -> Self {
        Self { timeout }
    }
}

/// Manages a Lua runtime for Niri.
///
/// This struct encapsulates the Lua runtime and provides methods for
/// executing scripts and registering components.
///
/// # Timeout Protection
///
/// This runtime implements wall-clock timeout protection using Luau's
/// `set_interrupt` callback. Runaway scripts are automatically terminated
/// after the configured timeout (default: 1 second).
pub struct LuaRuntime {
    lua: Lua,
    /// Event system for emitting Lua events from the compositor
    pub event_system: Option<EventSystem>,
    /// Pending configuration changes (for the new v2 API)
    pub pending_config: Option<SharedPendingChanges>,
    /// Queue of scheduled callbacks (stored as registry keys)
    scheduled_callbacks: Rc<RefCell<VecDeque<LuaRegistryKey>>>,
    /// Configured execution limits
    limits: ExecutionLimits,
    /// Shared deadline for interrupt callback (None = no active timeout)
    deadline: Rc<Cell<Option<Instant>>>,
}

impl LuaRuntime {
    /// Create a new Lua runtime with default execution limits.
    ///
    /// The default limits allow 1 second per Lua call, which is sufficient
    /// for most configuration and event handling code.
    ///
    /// # Errors
    ///
    /// Returns an error if the Lua runtime cannot be created.
    pub fn new() -> LuaResult<Self> {
        Self::new_with_limits(ExecutionLimits::default())
    }

    /// Create a new Lua runtime with custom execution limits.
    ///
    /// # Arguments
    ///
    /// * `limits` - Execution limits for script timeout protection
    ///
    /// # Errors
    ///
    /// Returns an error if the Lua runtime cannot be created.
    pub fn new_with_limits(limits: ExecutionLimits) -> LuaResult<Self> {
        let lua = Lua::new();

        // Set up standard library with appropriate restrictions
        lua.load_std_libs(LuaStdLib::ALL_SAFE)?;

        let scheduled_callbacks = Rc::new(RefCell::new(VecDeque::new()));
        let deadline = Rc::new(Cell::new(None::<Instant>));

        // Set up Luau interrupt callback for timeout protection
        if limits.timeout > Duration::ZERO {
            let deadline_clone = deadline.clone();
            lua.set_interrupt(move |_lua| {
                if let Some(dl) = deadline_clone.get() {
                    if Instant::now() > dl {
                        // Return an error to terminate execution
                        return Err(LuaError::external("Script execution timeout"));
                    }
                }
                Ok(LuaVmState::Continue)
            });
        }

        Ok(Self {
            lua,
            event_system: None,
            pending_config: None,
            scheduled_callbacks,
            limits,
            deadline,
        })
    }

    /// Get the current execution limits.
    pub fn limits(&self) -> &ExecutionLimits {
        &self.limits
    }

    /// Set the deadline for the current execution.
    ///
    /// This is called internally before executing Lua code.
    fn set_deadline(&self) {
        if self.limits.timeout > Duration::ZERO {
            self.deadline.set(Some(Instant::now() + self.limits.timeout));
        }
    }

    /// Clear the deadline after execution completes.
    fn clear_deadline(&self) {
        self.deadline.set(None);
    }

    /// Execute a callback with timeout protection.
    ///
    /// Sets up the deadline before execution and clears it afterward.
    /// If the callback exceeds the timeout, it returns a timeout error.
    ///
    /// # Arguments
    ///
    /// * `callback` - The Lua function to execute
    /// * `args` - Arguments to pass to the callback
    ///
    /// # Returns
    ///
    /// The result of calling the callback, or an error if it times out.
    pub fn call_with_timeout<R: mlua::FromLuaMulti>(
        &self,
        callback: &LuaFunction,
        args: impl mlua::IntoLuaMulti,
    ) -> LuaResult<R> {
        self.set_deadline();
        let result = callback.call::<R>(args);
        self.clear_deadline();
        result
    }

    /// Execute Lua code with timeout protection.
    ///
    /// Sets up the deadline before execution and clears it afterward.
    ///
    /// # Arguments
    ///
    /// * `code` - The Lua code to execute
    ///
    /// # Returns
    ///
    /// The result of evaluating the code, or an error if it times out.
    pub fn eval_with_timeout<R: mlua::FromLua>(&self, code: &str) -> LuaResult<R> {
        self.set_deadline();
        let result = self.lua.load(code).eval::<R>();
        self.clear_deadline();
        result
    }

    /// Initialize the scheduler API, registering `niri.schedule()`.
    ///
    /// This allows Lua scripts to defer callback execution to the next
    /// compositor event loop iteration, preventing long-running handlers
    /// from blocking frame rendering.
    ///
    /// # Example
    ///
    /// ```lua
    /// niri.schedule(function()
    ///     -- This runs on the next event loop iteration
    ///     print("Deferred execution")
    /// end)
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if the scheduler cannot be initialized.
    pub fn init_scheduler(&self) -> LuaResult<()> {
        let niri_table: LuaTable = self.lua.globals().get("niri")?;
        let queue = self.scheduled_callbacks.clone();

        let schedule_fn = self.lua.create_function(move |lua, callback: LuaFunction| {
            let mut q = queue.borrow_mut();

            // Enforce queue size limit
            if q.len() >= MAX_QUEUE_SIZE {
                return Err(LuaError::external(format!(
                    "Schedule queue full (max {} callbacks)",
                    MAX_QUEUE_SIZE
                )));
            }

            // Store callback in registry so it persists
            let key = lua.create_registry_value(callback)?;
            q.push_back(key);

            Ok(())
        })?;

        niri_table.set("schedule", schedule_fn)?;
        Ok(())
    }

    /// Execute scheduled callbacks with a limit per cycle.
    ///
    /// Returns the number of callbacks executed and any errors encountered.
    /// Callbacks scheduled during this flush may execute in the same cycle
    /// up to `MAX_CALLBACKS_PER_FLUSH` total.
    ///
    /// This should be called from the compositor's refresh cycle.
    pub fn flush_scheduled(&self) -> (usize, Vec<LuaError>) {
        let mut executed = 0;
        let mut errors = Vec::new();

        // Execute up to limit, allowing newly scheduled callbacks within limit
        while executed < MAX_CALLBACKS_PER_FLUSH {
            let key = self.scheduled_callbacks.borrow_mut().pop_front();
            match key {
                Some(registry_key) => {
                    // Retrieve callback from registry
                    let callback: LuaFunction = match self.lua.registry_value(&registry_key) {
                        Ok(cb) => cb,
                        Err(e) => {
                            errors.push(e);
                            executed += 1;
                            continue;
                        }
                    };

                    // Clean up registry
                    if let Err(e) = self.lua.remove_registry_value(registry_key) {
                        log::warn!("Failed to remove scheduled callback from registry: {}", e);
                    }

                    // Execute the callback with timeout protection
                    match self.call_with_timeout::<()>(&callback, ()) {
                        Ok(()) => executed += 1,
                        Err(e) => {
                            log::error!("Scheduled Lua callback failed: {}", e);
                            errors.push(e);
                            executed += 1;
                        }
                    }
                }
                None => break, // Queue empty
            }
        }

        (executed, errors)
    }

    /// Check if there are pending scheduled callbacks.
    pub fn has_scheduled(&self) -> bool {
        !self.scheduled_callbacks.borrow().is_empty()
    }

    /// Get the number of pending scheduled callbacks.
    pub fn scheduled_count(&self) -> usize {
        self.scheduled_callbacks.borrow().len()
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
    /// If the config proxy is already initialized (from `init_empty_config_proxy` during
    /// config loading), this updates the existing proxy's current values from the config
    /// while keeping the same pending changes handle. This ensures the Lua-side proxy
    /// remains connected to the same pending changes object that IPC commands will read from.
    ///
    /// # Errors
    ///
    /// Returns an error if configuration proxy API registration fails.
    pub fn register_config_proxy_api(
        &mut self,
        config: &Config,
    ) -> LuaResult<SharedPendingChanges> {
        // If already initialized, update the existing proxy's current values
        // while keeping the same pending changes handle
        if let Some(ref pending) = self.pending_config {
            update_config_proxy_values(&self.lua, config)?;
            return Ok(pending.clone());
        }

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
                    log::debug!(
                        "Config load: capturing spawn {:?} as spawn_at_startup",
                        command
                    );
                    // Add to pending config changes as spawn_at_startup
                    let spawn_json = serde_json::json!(command);
                    let mut pending = pending_clone.lock().unwrap();
                    pending
                        .collection_additions
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
    /// This sets up event handling capabilities, allowing Lua scripts to
    /// register callbacks for compositor events.
    ///
    /// # Errors
    ///
    /// Returns an error if the event system cannot be initialized.
    #[allow(clippy::arc_with_non_send_sync)] // LuaFunction is !Send, but we only use this on main thread
    pub fn init_event_system(&mut self) -> LuaResult<()> {
        let handlers = Arc::new(std::sync::Mutex::new(EventHandlers::new()));

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

    /// Update the config proxy's cached values from the current config.
    ///
    /// This should be called after applying config changes (e.g., via IPC) to ensure
    /// the Lua-side config proxy reflects the current state of the Rust config.
    /// Without this, reading `niri.config.prefer_no_csd` after an IPC change would
    /// return the old cached value instead of the new value.
    ///
    /// # Arguments
    ///
    /// * `config` - The current config to sync from
    ///
    /// # Errors
    ///
    /// Returns an error if the config proxy cannot be updated.
    pub fn sync_config_from(&self, config: &Config) -> LuaResult<()> {
        update_config_proxy_values(&self.lua, config)
    }

    /// Load and execute a Lua script from a file.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read, the script fails to execute,
    /// or the script exceeds the execution timeout.
    pub fn load_file<P: AsRef<Path>>(&self, path: P) -> LuaResult<LuaValue> {
        let code = std::fs::read_to_string(path)
            .map_err(|e| LuaError::external(format!("Failed to read Lua file: {}", e)))?;

        self.set_deadline();
        let result = self.lua.load(&code).eval();
        self.clear_deadline();
        result
    }

    /// Load and execute a Lua script from a string.
    ///
    /// # Errors
    ///
    /// Returns an error if the script fails to parse, execute,
    /// or exceeds the execution timeout.
    pub fn load_string(&self, code: &str) -> LuaResult<LuaValue> {
        self.set_deadline();
        let result = self.lua.load(code).eval();
        self.clear_deadline();
        result
    }

    /// Execute Lua code and capture output (for REPL/CLI usage).
    ///
    /// Captures print() output and returns the result.
    /// Returns a tuple of (output, success).
    pub fn execute_string(&self, code: &str) -> (String, bool) {
        // Get or create the format_value function for pretty-printing
        let format_value: LuaFunction = self
            .lua
            .globals()
            .get::<LuaFunction>("__niri_format_value")
            .unwrap_or_else(|_| {
                // Fallback: create inline if not registered
                self.lua
                    .load(include_str!("format_value.lua"))
                    .eval()
                    .unwrap()
            });

        // Capture print output using format_value for inspection
        let original_print = self.lua.globals().get::<LuaFunction>("print");
        let output = std::rc::Rc::new(std::cell::RefCell::new(Vec::<String>::new()));
        let output_capture = output.clone();
        let format_value_clone = format_value.clone();

        let print_fn =
            self.lua
                .create_function(move |_, args: mlua::MultiValue| -> LuaResult<()> {
                    let mut parts = Vec::new();
                    for v in args.iter() {
                        let formatted: String = format_value_clone.call(v.clone())?;
                        parts.push(formatted);
                    }
                    output_capture.borrow_mut().push(parts.join("\t"));
                    Ok(())
                });

        if let Ok(pf) = print_fn {
            let _ = self.lua.globals().set("print", pf);
        }

        self.set_deadline();
        let result = self.lua.load(code).eval::<LuaValue>();
        self.clear_deadline();

        // Restore original print
        if let Ok(orig) = original_print {
            let _ = self.lua.globals().set("print", orig);
        }

        let (success, message) = match result {
            Ok(val) => {
                // Use format_value for all return values (like vim.print)
                let val_str = match &val {
                    LuaValue::Nil => String::new(),
                    _ => format_value.call::<String>(val).unwrap_or_default(),
                };
                (true, val_str)
            }
            Err(e) => (false, format!("Error: {}", e)),
        };

        let output_vec = output.borrow();
        let full_output = if output_vec.is_empty() {
            message
        } else if message.is_empty() {
            output_vec.join("\n")
        } else {
            format!("{}\n{}", output_vec.join("\n"), message)
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
        let changes = shared.lock().unwrap();
        let layout_gaps = changes.scalar_changes.get("layout.gaps");
        assert!(layout_gaps.is_some());
    }

    #[test]
    fn config_proxy_captures_nested_values() {
        let mut rt = LuaRuntime::new().unwrap();
        let shared = rt.init_empty_config_proxy().unwrap();

        // Set a deeply nested value
        rt.load_string("niri.config.layout.border.active.color = '#ff0000'")
            .unwrap();

        let changes = shared.lock().unwrap();
        let border_color = changes.scalar_changes.get("layout.border.active.color");
        assert!(border_color.is_some());
    }

    #[test]
    fn config_proxy_captures_bind_additions() {
        let mut rt = LuaRuntime::new().unwrap();
        let shared = rt.init_empty_config_proxy().unwrap();

        // Add a keybind through the collection API
        rt.load_string(
            r#"
            niri.config.binds:add({
                key = 'Mod+Return',
                action = 'spawn',
                args = { 'alacritty' }
            })
        "#,
        )
        .unwrap();

        let changes = shared.lock().unwrap();
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
        rt.load_string(
            r#"
            niri.config.spawn_at_startup:add({
                command = { 'waybar' }
            })
        "#,
        )
        .unwrap();

        let changes = shared.lock().unwrap();
        // Collection additions are keyed by collection name
        let spawns = changes.collection_additions.get("spawn_at_startup");
        assert!(spawns.is_some());
        assert!(!spawns.unwrap().is_empty());
    }

    // ========================================================================
    // Execution Timeout tests (Luau set_interrupt)
    // ========================================================================

    #[test]
    fn new_with_limits_creates_runtime() {
        let limits = ExecutionLimits::with_timeout(Duration::from_millis(100));
        let rt = LuaRuntime::new_with_limits(limits);
        assert!(rt.is_ok());
    }

    #[test]
    fn unlimited_limits_disables_timeout() {
        let rt = LuaRuntime::new_with_limits(ExecutionLimits::unlimited()).unwrap();
        assert_eq!(rt.limits().timeout, Duration::ZERO);
    }

    #[test]
    fn default_limits_are_reasonable() {
        let limits = ExecutionLimits::default();
        assert!(limits.timeout > Duration::ZERO);
        assert!(limits.timeout <= Duration::from_secs(10));
    }

    #[test]
    fn infinite_loop_times_out() {
        // Use very short timeout for fast test execution
        let limits = ExecutionLimits::with_timeout(Duration::from_millis(50));
        let rt = LuaRuntime::new_with_limits(limits).unwrap();

        // This should timeout, not hang forever
        let result = rt.eval_with_timeout::<LuaValue>("while true do end");
        // The interrupt handler returns an error containing "timeout"
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("timeout"),
            "Expected timeout error, got: {}",
            err_msg
        );
    }

    #[test]
    fn normal_script_completes_within_limits() {
        let limits = ExecutionLimits::with_timeout(Duration::from_secs(1));
        let rt = LuaRuntime::new_with_limits(limits).unwrap();

        // A simple script should complete fine
        let result = rt.eval_with_timeout::<i64>(
            "local sum = 0; for i = 1, 100 do sum = sum + i end; return sum",
        );
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 5050);
    }

    #[test]
    fn deadline_resets_between_calls() {
        let limits = ExecutionLimits::with_timeout(Duration::from_millis(500));
        let rt = LuaRuntime::new_with_limits(limits).unwrap();

        // First call uses some time
        let result1 =
            rt.eval_with_timeout::<LuaValue>("local sum = 0; for i = 1, 100 do sum = sum + i end");
        assert!(result1.is_ok());

        // Second call should also succeed (deadline was reset)
        let result2 =
            rt.eval_with_timeout::<LuaValue>("local sum = 0; for i = 1, 100 do sum = sum + i end");
        assert!(result2.is_ok());

        // Third call should also succeed
        let result3 =
            rt.eval_with_timeout::<LuaValue>("local sum = 0; for i = 1, 100 do sum = sum + i end");
        assert!(result3.is_ok());
    }

    #[test]
    fn load_string_respects_timeout() {
        let limits = ExecutionLimits::with_timeout(Duration::from_millis(50));
        let rt = LuaRuntime::new_with_limits(limits).unwrap();

        // Infinite loop should be interrupted with timeout error
        let result = rt.load_string("while true do end");
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("timeout"),
            "Expected timeout error, got: {}",
            err_msg
        );
    }

    #[test]
    fn scheduled_callbacks_respect_timeout() {
        let limits = ExecutionLimits::with_timeout(Duration::from_millis(50));
        let rt = LuaRuntime::new_with_limits(limits).unwrap();
        rt.load_string("niri = {}").unwrap();
        rt.init_scheduler().unwrap();

        // Schedule an infinite loop callback
        rt.load_string(
            r#"
            niri.schedule(function()
                while true do end
            end)
        "#,
        )
        .unwrap();

        // Flush should timeout the callback, not hang
        let (count, errors) = rt.flush_scheduled();
        assert_eq!(count, 1);
        // Should have an error from the timeout
        assert_eq!(errors.len(), 1);
        assert!(
            errors[0].to_string().contains("timeout"),
            "Expected timeout error, got: {}",
            errors[0]
        );
    }

    // ========================================================================
    // Scheduler tests (niri.schedule)
    // ========================================================================

    #[test]
    fn init_scheduler_registers_function() {
        let rt = LuaRuntime::new().unwrap();
        // Create niri table first
        rt.load_string("niri = {}").unwrap();
        rt.init_scheduler().unwrap();

        // Verify niri.schedule exists and is callable
        let result = rt.load_string("return type(niri.schedule) == 'function'");
        assert!(result.is_ok());
        match result.unwrap() {
            LuaValue::Boolean(b) => assert!(b, "niri.schedule should be a function"),
            _ => panic!("Expected boolean result"),
        }
    }

    #[test]
    fn schedule_queues_callback() {
        let rt = LuaRuntime::new().unwrap();
        rt.load_string("niri = {}").unwrap();
        rt.init_scheduler().unwrap();

        // Schedule a callback
        rt.load_string(
            r#"
            __scheduled_ran = false
            niri.schedule(function()
                __scheduled_ran = true
            end)
        "#,
        )
        .unwrap();

        // Should not have run yet
        let ran: bool = rt.inner().globals().get("__scheduled_ran").unwrap();
        assert!(!ran);

        // Should have one callback queued
        assert_eq!(rt.scheduled_count(), 1);
        assert!(rt.has_scheduled());
    }

    #[test]
    fn flush_scheduled_executes_callbacks() {
        let rt = LuaRuntime::new().unwrap();
        rt.load_string("niri = {}").unwrap();
        rt.init_scheduler().unwrap();

        rt.load_string(
            r#"
            __scheduled_ran = false
            niri.schedule(function()
                __scheduled_ran = true
            end)
        "#,
        )
        .unwrap();

        // Flush the queue
        let (count, errors) = rt.flush_scheduled();
        assert_eq!(count, 1);
        assert!(errors.is_empty());

        // Now it should have run
        let ran: bool = rt.inner().globals().get("__scheduled_ran").unwrap();
        assert!(ran);

        // Queue should be empty
        assert!(!rt.has_scheduled());
    }

    #[test]
    fn flush_scheduled_handles_errors() {
        let rt = LuaRuntime::new().unwrap();
        rt.load_string("niri = {}").unwrap();
        rt.init_scheduler().unwrap();

        // Schedule a callback that errors
        rt.load_string(
            r#"
            niri.schedule(function()
                error("intentional error")
            end)
        "#,
        )
        .unwrap();

        let (count, errors) = rt.flush_scheduled();
        assert_eq!(count, 1);
        assert_eq!(errors.len(), 1);
    }

    #[test]
    fn flush_scheduled_respects_limit() {
        let rt = LuaRuntime::new().unwrap();
        rt.load_string("niri = {}").unwrap();
        rt.init_scheduler().unwrap();

        // Schedule more callbacks than the limit
        rt.load_string(
            r#"
            __count = 0
            for i = 1, 20 do
                niri.schedule(function()
                    __count = __count + 1
                end)
            end
        "#,
        )
        .unwrap();

        assert_eq!(rt.scheduled_count(), 20);

        // First flush should execute up to MAX_CALLBACKS_PER_FLUSH (16)
        let (count1, _) = rt.flush_scheduled();
        assert_eq!(count1, 16);

        // Remaining 4 should still be queued
        assert_eq!(rt.scheduled_count(), 4);

        // Second flush should get the rest
        let (count2, _) = rt.flush_scheduled();
        assert_eq!(count2, 4);

        // Queue should be empty now
        assert!(!rt.has_scheduled());
    }

    #[test]
    fn scheduled_callbacks_can_schedule_more() {
        let rt = LuaRuntime::new().unwrap();
        rt.load_string("niri = {}").unwrap();
        rt.init_scheduler().unwrap();

        // Schedule a callback that schedules another
        rt.load_string(
            r#"
            __first_ran = false
            __second_ran = false
            niri.schedule(function()
                __first_ran = true
                niri.schedule(function()
                    __second_ran = true
                end)
            end)
        "#,
        )
        .unwrap();

        // First flush - should run first callback and queue second
        let (count1, _) = rt.flush_scheduled();
        assert!(count1 >= 1);

        let first_ran: bool = rt.inner().globals().get("__first_ran").unwrap();
        assert!(first_ran);

        // If limit allows, second may have run too; if not, flush again
        if rt.has_scheduled() {
            let (count2, _) = rt.flush_scheduled();
            assert!(count2 >= 1);
        }

        let second_ran: bool = rt.inner().globals().get("__second_ran").unwrap();
        assert!(second_ran);
    }
}
