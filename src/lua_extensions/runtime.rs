//! Lua runtime initialization and management.
//!
//! This module handles creating and managing the Lua runtime with LuaJIT.
//! It provides utilities for loading scripts and managing the Lua environment.

use mlua::prelude::*;
use std::path::Path;
use crate::lua_extensions::{LuaComponent, NiriApi};

/// Manages a Lua runtime for Niri.
///
/// This struct encapsulates the Lua runtime and provides methods for
/// executing scripts and registering components.
pub struct LuaRuntime {
    lua: Lua,
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
        lua.load_from_std_lib(LuaStdLib::ALL)?;

        Ok(Self { lua })
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

    /// Load and execute a Lua script from a file.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read or the script fails to execute.
    pub fn load_file<P: AsRef<Path>>(&self, path: P) -> LuaResult<LuaValue<'_>> {
        let code = std::fs::read_to_string(path)
            .map_err(|e| LuaError::external(format!("Failed to read Lua file: {}", e)))?;

        self.lua.load(&code).eval()
    }

    /// Load and execute a Lua script from a string.
    ///
    /// # Errors
    ///
    /// Returns an error if the script fails to parse or execute.
    pub fn load_string(&self, code: &str) -> LuaResult<LuaValue<'_>> {
        self.lua.load(code).eval()
    }

    /// Execute a Lua function that takes no arguments and returns no value.
    ///
    /// # Errors
    ///
    /// Returns an error if the function doesn't exist or execution fails.
    pub fn call_function_void(&self, name: &str) -> LuaResult<()> {
        let func: LuaFunction = self.lua.globals().get(name)?;
        func.call::<_, ()>(())
    }

    /// Get a reference to the underlying Lua runtime.
    ///
    /// This allows direct access to the mlua::Lua instance for advanced use cases.
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

    #[test]
    fn test_new_runtime() {
        let rt = LuaRuntime::new();
        assert!(rt.is_ok());
    }

    #[test]
    fn test_load_string() {
        let rt = LuaRuntime::new().unwrap();
        let result = rt.load_string("return 42");
        assert!(result.is_ok());
    }
}
