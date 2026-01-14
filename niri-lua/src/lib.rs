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
pub mod niri_api;
pub mod os_utils;
pub mod runtime;

// Tier 1: Foundation Layer - Core Lua scripting infrastructure
pub mod event_handlers;
pub mod event_system;
pub mod events_proxy;
pub mod module_loader;

// Tier 2: Configuration API
pub mod accessor_macros;
pub mod action_proxy;
pub mod collections;
pub mod config_accessors;
pub mod config_api;
pub mod config_dirty;
pub mod config_proxy;
pub mod config_state;
pub mod config_wrapper;
pub mod extractors;
pub mod lua_types;
pub mod parse_utils;
pub mod property_registry;
pub mod rule_api;
pub mod traits;

// Tier 3: Runtime State Access
pub mod event_data;
pub mod ipc_bridge;
pub mod runtime_api;
pub mod state_handle;

// API Schema (for LSP type generation and testing)
pub mod api_registry;
pub mod lua_api_schema;

// Tier 4: Async Primitives
pub mod callback_registry;
pub mod loop_api;
pub mod process;

// Testing utilities (only available in tests)
#[cfg(test)]
pub mod test_utils;

#[cfg(test)]
mod test_derive_macros;

// Tier 2 exports
pub use action_proxy::{register_action_proxy, ActionCallback, ActionProxy};
// Tier 4 exports
pub use callback_registry::{CallbackRegistry, SharedCallbackRegistry};
pub use config::{LuaConfig, LuaEvalResult};
pub use config_dirty::ConfigDirtyFlags;
pub use config_proxy::ConfigProxy;
pub use config_state::{ConfigState, DirtyFlag};
pub use config_wrapper::{register_config_wrapper, ConfigWrapper};
// Tier 3 exports
pub use event_data::{
    EventData, LayoutEventData, MonitorEventData, WindowEventData, WorkspaceEventData,
};
// Tier 1 exports
pub use event_handlers::EventHandlers;
pub use event_system::{EventSystem, SharedEventHandlers};
pub use events_proxy::{register_events_proxy, EventsProxy};
pub use loop_api::{
    create_timer_manager, fire_due_timers, register_loop_api, SharedTimerManager, TimerManager,
    TimerState,
};
pub use lua_types::{LuaAnimation, LuaFilter, LuaGesture, LuaWindowRule};
use mlua::prelude::*;
pub use niri_api::NiriApi;
pub use process::{
    create_process_manager, parse_spawn_opts, ProcessHandle, ProcessManager, SharedProcessManager,
    SpawnOpts,
};
pub use property_registry::{
    extract_bool, extract_integer, extract_number, extract_string, infer_dirty_flag,
    parse_enum_variant, type_error, validate_array_elements, PropertyDescriptor, PropertyGetter,
    PropertyRegistry, PropertySetter, PropertyType,
};
pub use runtime::LuaRuntime;
pub use runtime_api::register_runtime_api;
pub use state_handle::{CursorPosition, FocusMode, ReservedSpace, StateHandle};

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
