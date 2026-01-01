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
//! # Performance Optimization
//!
//! The runtime uses Luau's Compiler with optimization level 2 for:
//! - Function inlining
//! - Loop unrolling
//! - Constant folding
//! - Dead code elimination
//!
//! # Async Primitives
//!
//! The timeout mechanism is accessible via helper functions that work with any `&Lua`:
//! - `set_lua_deadline(lua)` - Activates timeout protection
//! - `clear_lua_deadline(lua)` - Deactivates timeout protection
//! - `call_with_lua_timeout(lua, callback, args)` - Executes a callback with timeout
//!
//! These helpers allow timeout protection in code paths that don't have access to
//! `LuaRuntime`, such as timer callbacks and event handlers.
//!
//! In addition to timeout protection, we provide:
//! - `niri.schedule(fn)` - defer work to next event loop iteration
//! - Worker threads (Phase 4) - offload heavy computation
//! - `niri.loop` timers (Phase 5) - time-based scheduling

use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::path::Path;
use std::rc::Rc;
use std::sync::Arc;
use std::time::{Duration, Instant};

use mlua::prelude::*;
use mlua::Compiler;
use niri_config::Config;

use crate::action_proxy::{register_action_proxy, ActionCallback};
use crate::config_api::ConfigApi;
use crate::config_proxy::ConfigProxy;
use crate::config_state::ConfigState;
use crate::config_wrapper::{register_config_wrapper, ConfigWrapper};
use crate::event_handlers::EventHandlers;
use crate::event_system::EventSystem;
use crate::events_proxy::register_events_proxy;
use crate::loop_api::{
    create_timer_manager, fire_due_timers, register_loop_api, SharedTimerManager,
};
use crate::process::{create_process_manager, CallbackPayload, SharedProcessManager};
use crate::property_registry::PropertyRegistry;
use crate::{CallbackRegistry, LuaComponent, NiriApi, SharedCallbackRegistry};

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

/// Timeout state stored in Lua's app data for access by helper functions.
///
/// This allows timeout protection in code paths that don't have access to
/// `LuaRuntime`, such as `fire_due_timers` and event emission.
#[derive(Clone)]
pub struct TimeoutState {
    /// The timeout duration (Duration::ZERO = disabled)
    pub timeout: Duration,
    /// Shared deadline cell (same as used by interrupt callback)
    pub deadline: Rc<Cell<Option<Instant>>>,
}

/// Set the deadline for Lua execution timeout.
///
/// This activates timeout protection for subsequent Lua calls.
/// The deadline is automatically checked by the interrupt callback.
///
/// # Arguments
///
/// * `lua` - The Lua context (must have been created with timeout support)
///
/// # Example
///
/// ```ignore
/// set_lua_deadline(lua);
/// let result = callback.call::<()>(());
/// clear_lua_deadline(lua);
/// ```
pub fn set_lua_deadline(lua: &Lua) {
    if let Some(state) = lua.app_data_ref::<TimeoutState>() {
        if state.timeout > Duration::ZERO {
            state.deadline.set(Some(Instant::now() + state.timeout));
        }
    }
}

/// Clear the deadline after Lua execution completes.
///
/// This should be called after every Lua call to reset the timeout state.
pub fn clear_lua_deadline(lua: &Lua) {
    if let Some(state) = lua.app_data_ref::<TimeoutState>() {
        state.deadline.set(None);
    }
}

/// Execute a Lua callback with timeout protection.
///
/// This is a convenience function that sets the deadline, calls the callback,
/// and clears the deadline afterward.
///
/// # Arguments
///
/// * `lua` - The Lua context
/// * `callback` - The function to call
/// * `args` - Arguments to pass to the function
///
/// # Returns
///
/// The result of calling the callback, or a timeout error.
pub fn call_with_lua_timeout<R: mlua::FromLuaMulti>(
    lua: &Lua,
    callback: &LuaFunction,
    args: impl mlua::IntoLuaMulti,
) -> LuaResult<R> {
    set_lua_deadline(lua);
    let result = callback.call::<R>(args);
    clear_lua_deadline(lua);
    result
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
///
/// # Performance Optimization
///
/// Uses Luau's Compiler with optimization level 2 for function inlining,
/// loop unrolling, and constant folding.
pub struct LuaRuntime {
    lua: Lua,
    /// Event system for emitting Lua events from the compositor
    pub event_system: Option<EventSystem>,
    /// New config wrapper with direct niri_config::Config access
    pub config_wrapper: Option<ConfigWrapper>,
    /// Timer manager for niri.loop timers
    pub timer_manager: Option<SharedTimerManager>,
    /// Process manager for managed spawning
    pub process_manager: Option<SharedProcessManager>,
    /// Callback registry for function callbacks
    pub callback_registry: Option<SharedCallbackRegistry>,
    /// Queue of scheduled callbacks (stored as registry keys)
    scheduled_callbacks: Rc<RefCell<VecDeque<LuaRegistryKey>>>,
    /// Configured execution limits
    limits: ExecutionLimits,
    /// Shared deadline for interrupt callback (None = no active timeout)
    deadline: Rc<Cell<Option<Instant>>>,
    /// Luau compiler with optimization enabled (shared with require function)
    compiler: Rc<RefCell<Compiler>>,
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
        // Set up standard library with appropriate restrictions
        let lua = Lua::new_with(LuaStdLib::ALL_SAFE, LuaOptions::default())?;

        let scheduled_callbacks = Rc::new(RefCell::new(VecDeque::new()));
        let deadline = Rc::new(Cell::new(None::<Instant>));

        // Store timeout state in app data for access by helper functions
        // (fire_due_timers, emit_event, etc.)
        lua.set_app_data(TimeoutState {
            timeout: limits.timeout,
            deadline: deadline.clone(),
        });

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

        // Create optimized compiler for Luau
        // Level 2 enables function inlining, loop unrolling, constant folding
        let compiler = Rc::new(RefCell::new(
            Compiler::new().set_optimization_level(2).set_debug_level(1), /* Keep line info for
                                                                           * error messages */
        ));

        // Register custom require function for module loading
        crate::module_loader::register_custom_require(&lua, compiler.clone())?;

        Ok(Self {
            lua,
            event_system: None,
            config_wrapper: None,
            timer_manager: None,
            process_manager: None,
            callback_registry: None,
            scheduled_callbacks,
            limits,
            deadline,
            compiler,
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
            self.deadline
                .set(Some(Instant::now() + self.limits.timeout));
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
    /// Uses the optimized compiler for better performance.
    ///
    /// # Arguments
    ///
    /// * `code` - The Lua code to execute
    ///
    /// # Returns
    ///
    /// The result of evaluating the code, or an error if it times out.
    pub fn eval_with_timeout<R: mlua::FromLua>(&self, code: &str) -> LuaResult<R> {
        // Compile with optimizations
        let bytecode = self.compiler.borrow().compile(code)?;

        self.set_deadline();
        let result = self.lua.load(bytecode).eval::<R>();
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

        let schedule_fn = self
            .lua
            .create_function(move |lua, callback: LuaFunction| {
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

    /// Initialize the loop API, registering `niri.loop` with timer functions.
    ///
    /// This allows Lua scripts to create timers for delayed or repeated execution,
    /// and to query monotonic time for timing operations.
    ///
    /// # Example
    ///
    /// ```lua
    /// -- Create a one-shot timer
    /// local timer = niri.loop.new_timer()
    /// timer:start(1000, 0, function()
    ///     print("Timer fired!")
    ///     timer:close()
    /// end)
    ///
    /// -- Get current time in milliseconds
    /// local now = niri.loop.now()
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if the loop API cannot be initialized.
    pub fn init_loop_api(&mut self) -> LuaResult<()> {
        let manager = create_timer_manager();
        register_loop_api(&self.lua, manager.clone())?;

        // Load Lua loop helpers (e.g., niri.loop.defer, niri.schedule_wrap)
        self.lua.load(include_str!("loop_helpers.lua")).exec()?;

        // Load state watch helper (niri.state.watch)
        self.lua.load(include_str!("state_watch.lua")).exec()?;

        self.timer_manager = Some(manager);
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

    /// Fire all due timers, executing their callbacks.
    ///
    /// Returns the number of timers fired and any errors encountered.
    /// This should be called from the compositor's refresh cycle.
    pub fn fire_timers(&self) -> (usize, Vec<LuaError>) {
        if let Some(ref manager) = self.timer_manager {
            fire_due_timers(&self.lua, manager)
        } else {
            (0, Vec::new())
        }
    }

    /// Fire process events, invoking registered callbacks.
    ///
    /// Returns the number of callbacks executed and any errors encountered.
    /// This should be called from the compositor's refresh cycle.
    pub fn fire_process_events(&self) -> (usize, Vec<LuaError>) {
        let mut executed = 0;
        let mut errors = Vec::new();

        if let Some(ref manager) = self.process_manager {
            let events = manager.lock().unwrap().process_events();

            for event in events.into_iter().take(MAX_CALLBACKS_PER_FLUSH) {
                if let Some(ref registry) = self.callback_registry {
                    match registry.get(&self.lua, event.callback_id) {
                        Ok(Some(callback)) => {
                            let result = match &event.payload {
                                CallbackPayload::Stdout(data) | CallbackPayload::Stderr(data) => {
                                    if event.text_mode {
                                        let text = String::from_utf8_lossy(data);
                                        self.call_with_timeout(&callback, (LuaValue::Nil, text))
                                    } else {
                                        self.call_with_timeout(
                                            &callback,
                                            (LuaValue::Nil, self.lua.create_string(data).unwrap()),
                                        )
                                    }
                                }
                                CallbackPayload::Exit(result) => {
                                    let table =
                                        result.to_lua_table(&self.lua, event.text_mode).unwrap();
                                    self.call_with_timeout(&callback, (table, LuaValue::Nil))
                                }
                            };

                            match result {
                                Ok(()) => executed += 1,
                                Err(e) => {
                                    log::error!("Process callback failed: {}", e);
                                    errors.push(e);
                                    executed += 1;
                                }
                            }

                            // For exit events, clean up callbacks
                            if let CallbackPayload::Exit(_) = &event.payload {
                                if let Some((stdout_id, stderr_id, exit_id)) =
                                    manager.lock().unwrap().get_callback_ids(event.handle_id)
                                {
                                    if let Some(id) = stdout_id {
                                        let _ = registry.unregister(id);
                                    }
                                    if let Some(id) = stderr_id {
                                        let _ = registry.unregister(id);
                                    }
                                    if let Some(id) = exit_id {
                                        let _ = registry.unregister(id);
                                    }
                                }
                            }
                        }
                        Ok(None) => {
                            log::warn!("Callback {} not found in registry", event.callback_id);
                        }
                        Err(e) => {
                            log::error!("Failed to get callback {}: {}", event.callback_id, e);
                            errors.push(e);
                        }
                    }
                }
            }
        }

        (executed, errors)
    }

    /// Process all pending Lua async work: fire due timers and flush scheduled callbacks.
    ///
    /// This is the main entry point for the compositor to drive Lua async execution.
    /// Should be called once per frame/refresh cycle.
    ///
    /// Returns (timers_fired, scheduled_executed, process_events_executed, errors).
    pub fn process_async(&self) -> (usize, usize, usize, Vec<LuaError>) {
        let mut all_errors = Vec::new();

        // Fire due timers first (they may schedule callbacks)
        let (timers_fired, timer_errors) = self.fire_timers();
        all_errors.extend(timer_errors);

        // Then flush scheduled callbacks
        let (scheduled_executed, scheduled_errors) = self.flush_scheduled();
        all_errors.extend(scheduled_errors);

        // Finally fire process events
        let (process_executed, process_errors) = self.fire_process_events();
        all_errors.extend(process_errors);

        (
            timers_fired,
            scheduled_executed,
            process_executed,
            all_errors,
        )
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

    /// Initialize the dynamic ConfigProxy API and attach it to `niri.config`.
    ///
    /// This sets up the PropertyRegistry with metadata from `Config::property_metadata()`,
    /// replaces placeholder accessors with real implementations via
    /// `register_config_accessors()`, initializes ConfigState in Lua app data, and
    /// attaches `ConfigProxy` as the `niri.config` global.
    pub fn init_config_proxy_api(&mut self, config: Config) -> LuaResult<()> {
        PropertyRegistry::init_from_config();

        let config_rc = Rc::new(RefCell::new(config));
        let dirty = Rc::new(RefCell::new(
            crate::config_dirty::ConfigDirtyFlags::default(),
        ));
        let state = Rc::new(RefCell::new(ConfigState::new(
            config_rc.clone(),
            dirty.clone(),
        )));
        self.lua.set_app_data(state);

        let globals = self.lua.globals();
        let niri: LuaTable = globals.get("niri")?;
        niri.set("config", ConfigProxy::default())?;

        let wrapper = ConfigWrapper::new_with_shared_state(config_rc, dirty);
        self.config_wrapper = Some(wrapper);

        Ok(())
    }

    /// Register the new config wrapper API.
    ///
    /// This provides direct access to niri_config::Config through the `niri.config` table.
    /// Changes are tracked via `ConfigDirtyFlags` and can be extracted after script execution.
    ///
    /// This is the new API that will replace the old `register_config_proxy_api` method.
    ///
    /// # Arguments
    ///
    /// * `config` - The initial Config to populate the wrapper with
    ///
    /// # Returns
    ///
    /// The ConfigWrapper handle that can be used to extract the config and dirty flags.
    ///
    /// # Errors
    ///
    /// Returns an error if configuration wrapper registration fails.
    pub fn register_config_wrapper_api(&mut self, config: Config) -> LuaResult<ConfigWrapper> {
        let dirty = Rc::new(RefCell::new(
            crate::config_dirty::ConfigDirtyFlags::default(),
        ));
        let wrapper = ConfigWrapper::new_with_shared_state(Rc::new(RefCell::new(config)), dirty);
        register_config_wrapper(&self.lua, wrapper.clone())?;
        self.config_wrapper = Some(wrapper.clone());

        PropertyRegistry::init_from_config();

        let state = Rc::new(RefCell::new(ConfigState::new(
            wrapper.config.clone(),
            wrapper.dirty.clone(),
        )));
        self.lua.set_app_data(state);

        let globals = self.lua.globals();
        let niri: LuaTable = globals.get("niri")?;
        niri.set("config_new", ConfigProxy::default())?;

        Ok(wrapper)
    }

    /// Initialize an empty config wrapper for initial script loading.
    ///
    /// This creates a config wrapper with default values that can be modified
    /// by the Lua script during initial loading.
    ///
    /// # Returns
    ///
    /// The ConfigWrapper handle that can be used to extract the config and dirty flags.
    ///
    /// # Errors
    ///
    /// Returns an error if configuration wrapper initialization fails.
    pub fn init_empty_config_wrapper(&mut self) -> LuaResult<ConfigWrapper> {
        PropertyRegistry::init_from_config();

        let config_rc = Rc::new(RefCell::new(Config::default()));
        let dirty = Rc::new(RefCell::new(
            crate::config_dirty::ConfigDirtyFlags::default(),
        ));

        let state = Rc::new(RefCell::new(ConfigState::new(
            config_rc.clone(),
            dirty.clone(),
        )));
        self.lua.set_app_data(state);

        let wrapper = ConfigWrapper::new_with_shared_state(config_rc, dirty);
        register_config_wrapper(&self.lua, wrapper.clone())?;
        self.config_wrapper = Some(wrapper.clone());
        Ok(wrapper)
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
    /// Register the runtime state API for querying compositor state.
    ///
    /// # Errors
    ///
    /// Returns an error if runtime API registration fails.
    pub fn register_runtime_api(&self) -> LuaResult<()> {
        crate::register_runtime_api(&self.lua)
    }

    /// Initialize the event system for this runtime.
    ///
    /// This sets up event handling capabilities, allowing Lua scripts to
    /// register callbacks for compositor events.
    ///
    /// # Errors
    ///
    /// Returns an error if the event system cannot be initialized.
    pub fn init_event_system(&mut self) -> LuaResult<()> {
        let handlers = Rc::new(RefCell::new(EventHandlers::new()));

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
    pub fn register_action_proxy(&mut self, callback: ActionCallback) -> LuaResult<()> {
        register_action_proxy(
            &self.lua,
            callback,
            self.process_manager.clone(),
            self.callback_registry.clone(),
        )?;
        Ok(())
    }

    pub fn set_state_handle(&self, handle: crate::StateHandle) {
        self.lua.set_app_data(handle);
    }

    /// Initialize the process manager for managed spawning.
    ///
    /// This enables `niri.action:spawn()` and `niri.action:spawn_sh()` to return
    /// `ProcessHandle` userdata when called with an options table.
    ///
    /// # Note
    ///
    /// This should be called before `register_action_proxy()` if you want
    /// managed spawning support.
    pub fn init_process_manager(&mut self) {
        self.process_manager = Some(create_process_manager());
        self.callback_registry = Some(Arc::new(CallbackRegistry::new()));
    }

    /// Get the process manager, if initialized.
    pub fn process_manager(&self) -> Option<&SharedProcessManager> {
        self.process_manager.as_ref()
    }

    /// Load and execute a Lua script from a file.
    ///
    /// Uses the optimized compiler for better performance.
    /// Sets the chunk name to the file's absolute path for proper error messages
    /// and enables relative `require()` resolution.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read, the script fails to execute,
    /// or the script exceeds the execution timeout.
    pub fn load_file<P: AsRef<Path>>(&self, path: P) -> LuaResult<LuaValue> {
        let path = path.as_ref();
        let code = std::fs::read_to_string(path)
            .map_err(|e| LuaError::external(format!("Failed to read Lua file: {}", e)))?;

        // Get absolute path for chunk name and current file context
        let absolute_path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        let path_str = absolute_path.to_string_lossy().to_string();

        // Set current file context for relative requires
        self.lua
            .globals()
            .set("__niri_current_file", path_str.as_str())?;

        // Compile with optimizations
        let bytecode = self.compiler.borrow().compile(&code)?;

        self.set_deadline();
        let result = self.lua.load(bytecode).set_name(&path_str).eval();
        self.clear_deadline();
        result
    }

    /// Load and execute a Lua script from a string.
    ///
    /// Uses the optimized compiler for better performance.
    ///
    /// # Errors
    ///
    /// Returns an error if the script fails to parse, execute,
    /// or exceeds the execution timeout.
    pub fn load_string(&self, code: &str) -> LuaResult<LuaValue> {
        // Compile with optimizations
        let bytecode = self.compiler.borrow().compile(code)?;

        self.set_deadline();
        let result = self.lua.load(bytecode).eval();
        self.clear_deadline();
        result
    }

    /// Execute Lua code and capture output (for REPL/CLI usage).
    ///
    /// Captures print() output and returns the result.
    /// Returns a tuple of (output, success).
    pub fn execute_string(&self, code: &str) -> (String, bool) {
        // Get the format_value function for pretty-printing.
        // This is always registered by NiriApi before REPL usage.
        let format_value: LuaFunction = self
            .lua
            .globals()
            .get::<LuaFunction>("__niri_format_value")
            .expect("__niri_format_value must be registered before REPL execution");

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

        // Compile with optimizations
        let bytecode = match self.compiler.borrow().compile(code) {
            Ok(bc) => bc,
            Err(e) => {
                // Restore original print before returning
                if let Ok(orig) = original_print {
                    let _ = self.lua.globals().set("print", orig);
                }
                return (format!("Error: {}", e), false);
            }
        };

        self.set_deadline();
        let result = self.lua.load(bytecode).eval::<LuaValue>();
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
        // Register NiriApi to get __niri_format_value (required for REPL)
        rt.register_component(|_, _| Ok(())).unwrap();
        let (output, success) = rt.execute_string("return 1 + 1");
        assert!(success);
        assert!(output.contains("2"));
    }

    #[test]
    fn execute_string_syntax_error() {
        let rt = LuaRuntime::new().unwrap();
        // Register NiriApi to get __niri_format_value (required for REPL)
        rt.register_component(|_, _| Ok(())).unwrap();
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

    #[test]
    fn with_timeout_constructor_works() {
        let limits = ExecutionLimits::with_timeout(Duration::from_millis(250));
        assert_eq!(limits.timeout, Duration::from_millis(250));

        let rt = LuaRuntime::new_with_limits(limits).unwrap();
        assert_eq!(rt.limits().timeout, Duration::from_millis(250));
    }

    #[test]
    fn call_with_timeout_executes_function() {
        let rt = LuaRuntime::new().unwrap();

        // Create a Lua function
        rt.load_string(
            r#"
            function add(a, b)
                return a + b
            end
        "#,
        )
        .unwrap();

        let func: LuaFunction = rt.inner().globals().get("add").unwrap();
        let result: i64 = rt.call_with_timeout(&func, (10, 20)).unwrap();
        assert_eq!(result, 30);
    }

    #[test]
    fn call_with_timeout_times_out_infinite_function() {
        let limits = ExecutionLimits::with_timeout(Duration::from_millis(50));
        let rt = LuaRuntime::new_with_limits(limits).unwrap();

        // Create a function that loops forever
        rt.load_string(
            r#"
            function infinite()
                while true do end
            end
        "#,
        )
        .unwrap();

        let func: LuaFunction = rt.inner().globals().get("infinite").unwrap();
        let result = rt.call_with_timeout::<()>(&func, ());

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("timeout"));
    }

    #[test]
    fn nested_lua_calls_respect_timeout() {
        let limits = ExecutionLimits::with_timeout(Duration::from_millis(50));
        let rt = LuaRuntime::new_with_limits(limits).unwrap();

        // Create nested functions where the inner one loops
        let result = rt.eval_with_timeout::<LuaValue>(
            r#"
            local function inner()
                while true do end
            end
            local function outer()
                inner()
            end
            outer()
        "#,
        );

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("timeout"));
    }

    #[test]
    fn recursive_calls_respect_timeout() {
        let limits = ExecutionLimits::with_timeout(Duration::from_millis(50));
        let rt = LuaRuntime::new_with_limits(limits).unwrap();

        // Infinite recursion (will timeout before stack overflow)
        let result = rt.eval_with_timeout::<LuaValue>(
            r#"
            local function recurse(n)
                return recurse(n + 1)
            end
            recurse(0)
        "#,
        );

        // Should timeout or error (stack overflow), either is acceptable
        assert!(result.is_err());
    }

    #[test]
    fn deadline_cleared_after_timeout_allows_next_call() {
        let limits = ExecutionLimits::with_timeout(Duration::from_millis(50));
        let rt = LuaRuntime::new_with_limits(limits).unwrap();

        // First call times out
        let result1 = rt.eval_with_timeout::<LuaValue>("while true do end");
        assert!(result1.is_err());

        // Next call should work fine (deadline was cleared)
        let result2 = rt.eval_with_timeout::<i64>("return 42");
        assert!(result2.is_ok());
        assert_eq!(result2.unwrap(), 42);
    }

    #[test]
    fn very_short_timeout_still_works() {
        // Even 1ms timeout should eventually fire
        let limits = ExecutionLimits::with_timeout(Duration::from_millis(1));
        let rt = LuaRuntime::new_with_limits(limits).unwrap();

        let result = rt.eval_with_timeout::<LuaValue>("while true do end");
        assert!(result.is_err());
    }

    #[test]
    fn timeout_error_message_is_descriptive() {
        let limits = ExecutionLimits::with_timeout(Duration::from_millis(50));
        let rt = LuaRuntime::new_with_limits(limits).unwrap();

        let result = rt.eval_with_timeout::<LuaValue>("while true do end");
        let err_msg = result.unwrap_err().to_string();

        // Error message should mention timeout
        assert!(
            err_msg.to_lowercase().contains("timeout"),
            "Error message should mention timeout, got: {}",
            err_msg
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

    #[test]
    fn fire_timers_executes_due_and_handles_errors() {
        let mut rt = LuaRuntime::new().unwrap();
        rt.load_string("niri = {}").unwrap();

        // Without init_loop_api, should return gracefully
        let (count, errors) = rt.fire_timers();
        assert_eq!(count, 0);
        assert!(errors.is_empty());

        // Initialize loop API
        rt.init_loop_api().unwrap();

        // Create immediate timer (0ms), long-delay timer (10s), and error timer
        rt.load_string(
            r#"
            __immediate_ran = false
            __delayed_ran = false

            local t1 = niri.loop.new_timer()
            t1:start(0, 0, function() __immediate_ran = true end)

            local t2 = niri.loop.new_timer()
            t2:start(10000, 0, function() __delayed_ran = true end)

            local t3 = niri.loop.new_timer()
            t3:start(0, 0, function() error("timer error") end)
        "#,
        )
        .unwrap();

        let (count, errors) = rt.fire_timers();
        assert_eq!(count, 2); // immediate + error timer fired
        assert_eq!(errors.len(), 1);
        assert!(errors[0].to_string().contains("timer error"));

        // Immediate ran, delayed did not
        let immediate: bool = rt.inner().globals().get("__immediate_ran").unwrap();
        let delayed: bool = rt.inner().globals().get("__delayed_ran").unwrap();
        assert!(immediate);
        assert!(!delayed);
    }

    #[test]
    fn fire_timers_errors_include_timer_context() {
        let mut rt = LuaRuntime::new().unwrap();
        rt.load_string("niri = {}").unwrap();
        rt.init_loop_api().unwrap();

        rt.load_string(
            r#"
            __timer = niri.loop.new_timer()
            __timer:start(0, 100, function()
                error("boom")
            end)
        "#,
        )
        .unwrap();

        let (count, errors) = rt.fire_timers();
        assert_eq!(count, 1);
        assert_eq!(errors.len(), 1);

        let msg = errors[0].to_string();
        assert!(msg.contains("timer_id="));
        assert!(msg.contains("repeat=100ms"));
        assert!(msg.contains("boom"));
    }

    #[test]
    fn loop_defer_executes_and_auto_closes() {
        let mut rt = LuaRuntime::new().unwrap();
        rt.load_string("niri = {}").unwrap();
        rt.init_loop_api().unwrap();

        rt.load_string(
            r#"
            __fired = false
            __timer = niri.loop.defer(function()
                __fired = true
            end, 10)
        "#,
        )
        .unwrap();

        std::thread::sleep(std::time::Duration::from_millis(20));

        let (count, errors) = rt.fire_timers();
        assert_eq!(count, 1);
        assert!(errors.is_empty());

        let fired: bool = rt.inner().globals().get("__fired").unwrap();
        assert!(fired);

        let active: bool = rt
            .inner()
            .load("return __timer:is_active()")
            .eval()
            .unwrap();
        assert!(!active);

        // Subsequent fires should do nothing
        let (count2, errors2) = rt.fire_timers();
        assert_eq!(count2, 0);
        assert!(errors2.is_empty());
    }

    #[test]
    fn loop_defer_can_be_cancelled_before_firing() {
        let mut rt = LuaRuntime::new().unwrap();
        rt.load_string("niri = {}").unwrap();
        rt.init_loop_api().unwrap();

        rt.load_string(
            r#"
            __fired = false
            __timer = niri.loop.defer(function()
                __fired = true
            end, 50)
            __timer:stop()
        "#,
        )
        .unwrap();

        std::thread::sleep(std::time::Duration::from_millis(60));

        let (count, errors) = rt.fire_timers();
        assert_eq!(count, 0);
        assert!(errors.is_empty());

        let fired: bool = rt.inner().globals().get("__fired").unwrap();
        assert!(!fired);

        let active: bool = rt
            .inner()
            .load("return __timer:is_active()")
            .eval()
            .unwrap();
        assert!(!active);
    }

    #[test]
    fn process_async_combines_timers_and_scheduled_with_chaining() {
        let mut rt = LuaRuntime::new().unwrap();
        rt.load_string("niri = {}").unwrap();
        rt.init_scheduler().unwrap();
        rt.init_loop_api().unwrap();

        // Schedule a callback that schedules another callback
        rt.load_string(
            r#"
            __step1 = false
            __step2 = false
            niri.schedule(function()
                __step1 = true
                niri.schedule(function()
                    __step2 = true
                end)
            end)
        "#,
        )
        .unwrap();

        // process_async runs all scheduled callbacks including newly queued ones
        // So both step1 and step2 should run in a single call
        let (timers, scheduled, _process, errors) = rt.process_async();
        assert_eq!(timers, 0);
        assert_eq!(scheduled, 2); // Both callbacks run in one pass
        assert!(errors.is_empty());

        let step1: bool = rt.inner().globals().get("__step1").unwrap();
        let step2: bool = rt.inner().globals().get("__step2").unwrap();
        assert!(step1);
        assert!(step2); // Both completed

        // Subsequent call should have nothing to do
        let (timers2, scheduled2, _process2, errors2) = rt.process_async();
        assert_eq!(timers2, 0);
        assert_eq!(scheduled2, 0);
        assert!(errors2.is_empty());
    }

    #[test]
    fn fire_process_events_without_manager() {
        let rt = LuaRuntime::new().unwrap();

        // Without process manager initialized, should return zero
        let (count, errors) = rt.fire_process_events();
        assert_eq!(count, 0);
        assert!(errors.is_empty());
    }

    #[test]
    fn fire_process_events_with_initialized_manager() {
        let mut rt = LuaRuntime::new().unwrap();
        rt.init_process_manager();

        // Without any processes, should return zero
        let (count, errors) = rt.fire_process_events();
        assert_eq!(count, 0);
        assert!(errors.is_empty());
    }

    #[test]
    fn process_async_includes_process_events() {
        let mut rt = LuaRuntime::new().unwrap();
        rt.load_string("niri = {}").unwrap();
        rt.init_scheduler().unwrap();
        rt.init_loop_api().unwrap();
        rt.init_process_manager();

        // This test verifies that process_async now returns 4 values including process events
        let (timers, scheduled, process, errors) = rt.process_async();
        assert_eq!(timers, 0);
        assert_eq!(scheduled, 0);
        assert_eq!(process, 0);
        assert!(errors.is_empty());
    }
}
