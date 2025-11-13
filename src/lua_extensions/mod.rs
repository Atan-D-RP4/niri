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

use mlua::prelude::*;
pub use config::LuaConfig;
pub use config_converter::apply_lua_config;
pub use niri_api::NiriApi;
pub use runtime::LuaRuntime;

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
        assert!(lua.globals().raw_get::<_, mlua::Value>("_G").is_ok());
    }
}
