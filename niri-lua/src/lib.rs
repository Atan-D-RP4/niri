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
pub mod module_loader;
pub mod plugin_system;
pub mod event_emitter;
pub mod hot_reload;

// Tier 2: Configuration API
pub mod lua_types;
pub mod validators;
pub mod extractors;
pub mod config_api;

// Tier 3: Runtime State Access
pub mod ipc_bridge;
pub mod runtime_api;

use mlua::prelude::*;
pub use config::LuaConfig;
pub use config_converter::apply_lua_config;
pub use niri_api::NiriApi;
pub use runtime::LuaRuntime;

// Tier 1 exports
pub use event_emitter::EventEmitter;
pub use hot_reload::HotReloader;
pub use module_loader::ModuleLoader;
pub use plugin_system::PluginManager;

// Tier 2 exports
pub use lua_types::{LuaAnimation, LuaFilter, LuaWindowRule, LuaGesture};

// Tier 3 exports
pub use runtime_api::{CompositorState, RuntimeApi, register_runtime_api};

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
    fn test_lua_runtime_creation() {
        let lua = Lua::new();
        let g: mlua::Value = lua.globals().raw_get("_G").unwrap();
        assert!(!matches!(g, mlua::Value::Nil));
    }
}
