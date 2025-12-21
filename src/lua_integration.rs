//! Lua configuration and runtime integration.
//!
//! This module consolidates all Lua-related setup code to reduce churn in `main.rs`
//! and provide a clean interface for Lua integration.
//!
//! # Overview
//!
//! The Lua integration supports two initialization patterns:
//!
//! ## Pattern 1: Two-Phase Init (Recommended - Neovim-style)
//!
//! This pattern allows Lua config to query compositor State during evaluation:
//!
//! ```ignore
//! // Phase 1: Create runtime (before State exists)
//! let lua_runtime = lua_integration::create_lua_runtime(&config_path);
//!
//! // ... create State ...
//!
//! // Phase 2: Setup APIs (State now exists)
//! lua_integration::setup_runtime(&mut state, lua_runtime, &event_loop, action_tx);
//!
//! // Phase 3: Evaluate config (niri.state.* queries work)
//! let eval_result = lua_integration::evaluate_lua_config(&mut state, &config_path);
//!
//! // Phase 4: Apply changes and execute pending actions
//! lua_integration::apply_lua_config(&mut state, &eval_result);
//! lua_integration::execute_pending_actions(&mut state, eval_result.pending_actions);
//! ```
//!
//! ## Pattern 2: Single-Phase Init (Legacy)
//!
//! This pattern loads and evaluates config in one step (State queries not available):
//!
//! ```ignore
//! let lua_result = lua_integration::load_lua_config(&config_path, &mut config);
//! // ... create State ...
//! lua_integration::setup_runtime(&mut state, lua_result.runtime, &event_loop, action_tx);
//! lua_integration::execute_pending_actions(&mut state, lua_result.pending_actions);
//! ```

use std::path::PathBuf;

use calloop::LoopHandle;
use niri_config::{Config, ConfigPath};
use niri_lua::{LuaConfig, LuaEvalResult, LuaRuntime, RuntimeApi};
use tracing::{info, warn};

use crate::niri::State;

/// Result of loading Lua configuration.
#[derive(Default)]
pub struct LuaLoadResult {
    /// The Lua runtime, if config was successfully loaded.
    pub runtime: Option<LuaRuntime>,
    /// Pending actions collected during config loading (e.g., `niri.action:spawn()` calls).
    pub pending_actions: Vec<niri_ipc::Action>,
}

/// Determines which Lua config files to try loading based on the config path.
fn get_lua_config_paths(config_path: &ConfigPath) -> Vec<PathBuf> {
    match config_path {
        // Explicit Lua config file specified via -c flag
        ConfigPath::Explicit(path) if ConfigPath::is_lua_config(path) => {
            vec![path.clone()]
        }
        // Regular config path - check for niri.lua or init.lua in the config directory
        ConfigPath::Regular { user_path, .. } => {
            if let Some(config_dir) = user_path.parent() {
                vec![config_dir.join("niri.lua"), config_dir.join("init.lua")]
            } else {
                Vec::new()
            }
        }
        // Explicit non-Lua config - don't try to load Lua
        ConfigPath::Explicit(_) => Vec::new(),
    }
}

// ============================================================================
// Two-Phase Initialization API (Neovim-style)
// ============================================================================

/// Phase 1: Create a Lua runtime without evaluating user config.
///
/// This creates the Lua VM and registers base APIs but does NOT load or evaluate
/// any user config file. Call this BEFORE creating compositor State.
///
/// # Arguments
///
/// * `config_path` - The config path specification to find Lua config files
///
/// # Returns
///
/// `Some(LuaConfig)` if a Lua config file exists, `None` otherwise.
pub fn create_lua_runtime(config_path: &ConfigPath) -> Option<LuaConfig> {
    let lua_files = get_lua_config_paths(config_path);

    for lua_file in lua_files {
        if !lua_file.exists() {
            continue;
        }

        info!(
            "Creating Lua runtime for {} (two-phase init)",
            lua_file.display()
        );

        match LuaConfig::create_runtime() {
            Ok(lua_config) => {
                return Some(lua_config);
            }
            Err(e) => {
                warn!("Failed to create Lua runtime: {}", e);
                continue;
            }
        }
    }

    info!("No Lua config found, skipping Lua runtime creation");
    None
}

/// Phase 2: Setup the Lua runtime with APIs (two-phase version).
///
/// This registers RuntimeApi and other APIs to the runtime. Call this AFTER
/// State is created so that state queries will work.
///
/// Note: This does NOT store the runtime in state - the caller should pass
/// the LuaConfig to `evaluate_lua_config()` next.
///
/// # Arguments
///
/// * `lua_config` - The LuaConfig created by `create_lua_runtime()`
/// * `event_loop` - The event loop handle for RuntimeApi
/// * `action_tx` - Channel sender for Lua actions
pub fn setup_lua_config_apis(
    lua_config: &mut LuaConfig,
    event_loop: &LoopHandle<'static, State>,
    action_tx: calloop::channel::Sender<niri_ipc::Action>,
) {
    let runtime = lua_config.runtime_mut();

    // Register runtime API for state queries
    let runtime_api = RuntimeApi::new(event_loop.clone());
    if let Err(e) = runtime.register_runtime_api(runtime_api) {
        warn!("Failed to register Lua runtime API: {}", e);
    }

    // Register config wrapper API for reactive config access
    // Pass a default config - the wrapper will be updated when apply_config_wrapper_changes is
    // called
    if let Err(e) = runtime.register_config_wrapper_api(Config::default()) {
        warn!("Failed to register Lua config wrapper API: {}", e);
    }

    // Register action callback for IPC Lua execution
    let action_callback: niri_lua::ActionCallback =
        std::sync::Arc::new(move |action: niri_ipc::Action| {
            action_tx
                .send(action)
                .map_err(|e| mlua::Error::runtime(format!("Failed to send action: {}", e)))
        });

    if let Err(e) = runtime.register_action_proxy(action_callback) {
        warn!("Failed to register Lua action proxy: {}", e);
    }
}

/// Phase 3: Evaluate Lua configuration with State available for queries.
///
/// This evaluates the user's Lua config file AFTER State exists and RuntimeApi
/// has been registered. This allows Lua config scripts to call `niri.state.windows()`,
/// `niri.state.workspaces()`, etc. during initial configuration.
///
/// After evaluation, the LuaConfig is converted to a LuaRuntime and stored in state.
///
/// # Arguments
///
/// * `state` - The compositor state (runtime will be stored here after evaluation)
/// * `lua_config` - The LuaConfig to evaluate
/// * `config_path` - The config path specification to find Lua config files
///
/// # Returns
///
/// A `LuaEvalResult` containing config changes, pending actions, and any errors.
pub fn evaluate_lua_config(
    state: &mut State,
    mut lua_config: LuaConfig,
    config_path: &ConfigPath,
) -> LuaEvalResult {
    // Skip if already evaluated
    if lua_config.is_evaluated() {
        // Store the runtime and return empty
        state.niri.lua_runtime = Some(lua_config.into_runtime());
        return LuaEvalResult::empty();
    }

    let lua_files = get_lua_config_paths(config_path);

    for lua_file in lua_files {
        if !lua_file.exists() {
            continue;
        }

        info!(
            "Evaluating Lua config from {} (two-phase init)",
            lua_file.display()
        );

        match lua_config.evaluate_file(&lua_file) {
            Ok(result) => {
                if result.has_errors() {
                    for error in &result.errors {
                        warn!("Lua config evaluation error: {}", error);
                    }
                    // Show config error notification
                    state.niri.config_error_notification.show();
                }

                info!("Lua config evaluated successfully");

                // Store the runtime in state
                state.niri.lua_runtime = Some(lua_config.into_runtime());

                return result;
            }
            Err(e) => {
                warn!("Failed to evaluate Lua config: {}", e);
                state.niri.config_error_notification.show();
                // Store the runtime anyway (evaluation failed but runtime is valid)
                state.niri.lua_runtime = Some(lua_config.into_runtime());
                return LuaEvalResult::with_error(e);
            }
        }
    }

    // No Lua file found to evaluate - store the runtime anyway
    state.niri.lua_runtime = Some(lua_config.into_runtime());
    LuaEvalResult::empty()
}

/// Apply Lua config changes to the compositor state.
///
/// This extracts configuration changes from the Lua evaluation result and applies
/// them to the compositor's config.
///
/// # Arguments
///
/// * `state` - The compositor state to update
/// * `result` - The evaluation result containing config changes
pub fn apply_lua_config(state: &mut State, result: &LuaEvalResult) {
    let Some(ref wrapper) = result.config_wrapper else {
        info!("No Lua config changes to apply");
        return;
    };

    let dirty = wrapper.take_dirty_flags();
    if !dirty.any() {
        info!("No Lua config changes to apply (no dirty flags)");
        return;
    }

    let lua_config = wrapper.extract_config();

    // Log changes
    let binds_before = state.niri.config.borrow().binds.0.len();
    let startup_before = state.niri.config.borrow().spawn_at_startup.len();

    // Replace the config
    *state.niri.config.borrow_mut() = lua_config;

    let binds_after = state.niri.config.borrow().binds.0.len();
    let startup_after = state.niri.config.borrow().spawn_at_startup.len();

    info!(
        "Applied Lua config changes: {} binds (+{}), {} startup commands (+{})",
        binds_after,
        binds_after.saturating_sub(binds_before),
        startup_after,
        startup_after.saturating_sub(startup_before)
    );
}

// ============================================================================
// Legacy Single-Phase API (backward compatibility)
// ============================================================================

/// Loads Lua configuration and applies it to the config (legacy single-phase).
///
/// Tries to load Lua config from the appropriate paths based on `config_path`.
/// If successful, merges the Lua config into `config` and returns the runtime
/// and pending actions.
///
/// # Arguments
///
/// * `config_path` - The config path specification (explicit or default)
/// * `config` - The config to merge Lua settings into
///
/// # Returns
///
/// A `LuaLoadResult` containing the runtime and pending actions, or defaults
/// if no Lua config was found/loaded.
pub fn load_lua_config(config_path: &ConfigPath, config: &mut Config) -> LuaLoadResult {
    let lua_files_to_try = get_lua_config_paths(config_path);

    for lua_file in lua_files_to_try {
        if !lua_file.exists() {
            continue;
        }

        match LuaConfig::from_file(&lua_file) {
            Ok(lua_config) => {
                info!("Loaded Lua config from {}", lua_file.display());

                // Log bind count before applying Lua config
                let binds_before = config.binds.0.len();
                let startup_before = config.spawn_at_startup.len();
                info!(
                    "Config state BEFORE Lua application: {} binds, {} startup commands",
                    binds_before, startup_before
                );

                // Extract and apply the Lua config
                // Only replace config if Lua actually modified something
                if let Some(wrapper) = lua_config.config_wrapper() {
                    let lua_config_obj = wrapper.extract_config();
                    let dirty = wrapper.take_dirty_flags();

                    if dirty.any() {
                        // Merge the Lua config into the existing config
                        // For now, we replace the entire config with the Lua one
                        // TODO: Consider selective merging for hybrid KDL+Lua configs
                        *config = lua_config_obj;
                    }
                }

                let binds_after = config.binds.0.len();
                let startup_after = config.spawn_at_startup.len();
                info!(
                    "Applied Lua config changes: {} binds (+{}), {} startup commands (+{})",
                    binds_after,
                    binds_after.saturating_sub(binds_before),
                    startup_after,
                    startup_after.saturating_sub(startup_before)
                );

                // Take pending actions before consuming the LuaConfig
                let pending_actions = lua_config.take_pending_actions();
                if !pending_actions.is_empty() {
                    info!(
                        "Collected {} pending Lua actions for execution",
                        pending_actions.len()
                    );
                }

                // Convert to runtime
                let runtime = lua_config.into_runtime();

                return LuaLoadResult {
                    runtime: Some(runtime),
                    pending_actions,
                };
            }
            Err(e) => {
                warn!(
                    "Failed to load Lua config from {}: {}",
                    lua_file.display(),
                    e
                );
            }
        }
    }

    LuaLoadResult::default()
}

/// Sets up the Lua runtime with all necessary APIs and callbacks.
///
/// This registers:
/// - Runtime API for state queries (windows, workspaces, etc.)
/// - Config wrapper API for reactive config access
/// - Action callback for executing actions from Lua
///
/// # Arguments
///
/// * `state` - The compositor state (runtime will be stored here)
/// * `runtime` - The Lua runtime to set up (if Some)
/// * `event_loop` - The event loop handle for registering sources
/// * `action_tx` - Channel sender for Lua actions
pub fn setup_runtime(
    state: &mut State,
    runtime: Option<LuaRuntime>,
    event_loop: &LoopHandle<'static, State>,
    action_tx: calloop::channel::Sender<niri_ipc::Action>,
) {
    // Store the Lua runtime in the compositor state
    state.niri.lua_runtime = runtime;

    // Register APIs if runtime is present
    if let Some(ref mut runtime) = state.niri.lua_runtime {
        // Register runtime API for state queries
        let runtime_api = RuntimeApi::new(event_loop.clone());
        if let Err(e) = runtime.register_runtime_api(runtime_api) {
            warn!("Failed to register Lua runtime API: {}", e);
        }

        // Register config wrapper API for reactive config access
        // Pass a default config - the wrapper will be updated when apply_config_wrapper_changes is
        // called
        if let Err(e) = runtime.register_config_wrapper_api(Config::default()) {
            warn!("Failed to register Lua config wrapper API: {}", e);
        }

        // Register action callback for IPC Lua execution
        let action_callback: niri_lua::ActionCallback =
            std::sync::Arc::new(move |action: niri_ipc::Action| {
                action_tx
                    .send(action)
                    .map_err(|e| mlua::Error::runtime(format!("Failed to send action: {}", e)))
            });

        if let Err(e) = runtime.register_action_proxy(action_callback) {
            warn!("Failed to register Lua action proxy: {}", e);
        }
    }
}

/// Creates the Lua action channel and registers it with the event loop.
///
/// Returns the sender that should be passed to `setup_runtime`.
///
/// Actions received through this channel will:
/// 1. Advance animations to ensure smooth visual feedback
/// 2. Execute the action via `state.do_action()`
pub fn create_action_channel(
    event_loop: &LoopHandle<'static, State>,
) -> calloop::channel::Sender<niri_ipc::Action> {
    let (tx, rx) = calloop::channel::channel::<niri_ipc::Action>();

    event_loop
        .insert_source(rx, |event, _, state| {
            if let calloop::channel::Event::Msg(action) = event {
                let action = niri_config::Action::from(action);
                // Advance animations before executing the action to ensure
                // smooth visual feedback for actions triggered from Lua
                state.niri.advance_animations();
                info!("Executing Lua action: {:?}", action);
                state.do_action(action, false);
            }
        })
        .expect("Failed to insert Lua action channel source");

    tx
}

/// Executes pending actions that were collected during config loading.
///
/// These are actions like `niri.action:spawn()` that were called during
/// config evaluation but couldn't be executed until the compositor was ready.
pub fn execute_pending_actions(state: &mut State, pending_actions: Vec<niri_ipc::Action>) {
    for action in pending_actions {
        info!("Executing pending Lua action: {:?}", action);
        state.do_action(action.into(), false);
    }
}

/// Returns whether Lua config is active (runtime is present).
///
/// This is useful for conditionally setting up KDL config watchers.
pub fn is_lua_config_active(state: &State) -> bool {
    state.niri.lua_runtime.is_some()
}
