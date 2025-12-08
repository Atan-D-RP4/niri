//! Lua scripting support for Niri through mlua bindings.
//!
//! This module provides Lua scripting capabilities to Niri, inspired by the Astra project.
//! It uses mlua with the Luau Lua dialect for type safety and better performance.
//!
//! # Example
//!
//! ```lua
//! -- Access Niri API functions from Lua
//! niri.log("Hello from Lua!")
//! ```

pub mod config;
pub mod config_converter;
pub mod niri_api;
pub mod runtime;

// Tier 1: Foundation Layer - Core Lua scripting infrastructure
pub mod event_emitter;
pub mod event_handlers;
pub mod event_system;
pub mod events_proxy;
pub mod hot_reload;
pub mod module_loader;
pub mod plugin_system;

// Tier 2: Configuration API
pub mod action_proxy;
pub mod config_api;
pub mod config_proxy;
pub mod extractors;
pub mod lua_types;
pub mod validators;

// Tier 3: Runtime State Access
pub mod event_data;
pub mod ipc_bridge;
pub mod ipc_repl;
pub mod runtime_api;

// API Schema (for LSP type generation and testing)
pub mod api_registry;
pub mod lua_api_schema;

// Tier 4: Async Primitives
pub mod loop_api;

// Testing utilities (only available in tests)
#[cfg(test)]
pub mod test_utils;

// Tier 2 exports
pub use action_proxy::{register_action_proxy, ActionCallback, ActionProxy};
pub use config::LuaConfig;
pub use config_converter::apply_pending_lua_config;
pub use config_proxy::{
    create_shared_pending_changes, register_config_proxy_to_lua, ConfigCollectionProxy,
    ConfigProxy, ConfigSectionProxy, PendingConfigChanges, SharedPendingChanges,
};
// Tier 3 exports
pub use event_data::{
    EventData, LayoutEventData, MonitorEventData, WindowEventData, WorkspaceEventData,
};
// Tier 1 exports
pub use event_emitter::register_to_lua as register_event_emitter;
pub use event_handlers::EventHandlers;
pub use event_system::{EventSystem, SharedEventHandlers};
pub use events_proxy::{register_events_proxy, EventsProxy};
pub use hot_reload::HotReloader;
pub use ipc_repl::IpcLuaExecutor;
// Tier 4 exports
pub use loop_api::{
    create_timer_manager, fire_due_timers, register_loop_api, SharedTimerManager, TimerManager,
    TimerState,
};
pub use lua_types::{LuaAnimation, LuaFilter, LuaGesture, LuaWindowRule};
use mlua::prelude::*;
pub use module_loader::ModuleLoader;
pub use niri_api::NiriApi;
pub use plugin_system::PluginManager;
pub use runtime::LuaRuntime;
pub use runtime_api::{
    clear_event_context_state, register_runtime_api, set_event_context_state, CompositorState,
    RuntimeApi, StateSnapshot,
};

/// Trait for registering Lua components to the global context.
///
/// Implement this trait to add custom Lua functions and types to the runtime.
/// This follows the pattern established by Astra for extensibility.
pub trait LuaComponent {
    /// Register this component's functions and types to the Lua runtime.
    ///
    /// This is called during runtime initialization to set up all Lua bindings.
    fn register_to_lua<F>(lua: &Lua, action_callback: F) -> LuaResult<()>
    where
        F: Fn(String, Vec<String>) -> LuaResult<()> + 'static;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lua_runtime_creation() {
        let lua = Lua::new();
        let g: mlua::Value = lua.globals().raw_get("_G").unwrap();
        assert!(!matches!(g, mlua::Value::Nil));
    }
}
