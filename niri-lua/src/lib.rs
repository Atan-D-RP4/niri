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
pub mod config_api;
pub mod config_proxy;
pub mod extractors;
pub mod lua_types;
pub mod validators;
pub mod action_proxy;

// Tier 3: Runtime State Access
pub mod event_data;
pub mod ipc_bridge;
pub mod ipc_repl;
pub mod runtime_api;

// Testing utilities (only available in tests)
#[cfg(test)]
pub mod test_utils;

pub use config::LuaConfig;
pub use config_converter::apply_pending_lua_config;
// Tier 3 exports
pub use event_data::{
    EventData, LayoutEventData, MonitorEventData, WindowEventData, WorkspaceEventData,
};
// Tier 1 exports
pub use event_emitter::EventEmitter;
pub use event_handlers::EventHandlers;
pub use event_system::{EventSystem, SharedEventHandlers};
pub use events_proxy::{register_events_proxy, EventsProxy};
pub use hot_reload::HotReloader;
pub use ipc_repl::IpcLuaExecutor;
// Tier 2 exports
pub use config_proxy::{
    create_shared_pending_changes, register_config_proxy_to_lua, ConfigCollectionProxy,
    ConfigProxy, ConfigSectionProxy, PendingConfigChanges, SharedPendingChanges,
};
pub use lua_types::{LuaAnimation, LuaFilter, LuaGesture, LuaWindowRule};
pub use action_proxy::{register_action_proxy, ActionCallback, ActionProxy};
use mlua::prelude::*;
pub use module_loader::ModuleLoader;
pub use niri_api::NiriApi;
pub use plugin_system::PluginManager;
pub use runtime::LuaRuntime;
pub use runtime_api::{register_runtime_api, clear_event_context_state, set_event_context_state, CompositorState, RuntimeApi, StateSnapshot};

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
