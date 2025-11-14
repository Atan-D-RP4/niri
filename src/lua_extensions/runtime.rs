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
        lua.load_std_libs(LuaStdLib::ALL_SAFE)?;

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

    /// Check if a global variable exists in the Lua runtime.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the global variable to check
    pub fn has_global(&self, name: &str) -> bool {
        self.lua
            .globals()
            .get::<LuaValue>(name)
            .is_ok()
    }

    /// Get a string value from the Lua runtime globals.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the global variable
    ///
    /// # Returns
    ///
    /// Returns Ok(Some(value)) if the variable exists and is a string,
    /// Ok(None) if the variable doesn't exist, or an error if it exists but isn't a string.
    pub fn get_global_string_opt(&self, name: &str) -> LuaResult<Option<String>> {
        match self.lua.globals().get::<LuaValue>(name) {
            Ok(LuaValue::Nil) => Ok(None),
            Ok(LuaValue::String(s)) => Ok(Some(s.to_string_lossy().to_string())),
            Ok(_) => Err(LuaError::external(format!(
                "Global '{}' is not a string",
                name
            ))),
            Err(_) => Ok(None),
        }
    }

    /// Get a boolean value from the Lua runtime globals.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the global variable
    ///
    /// # Returns
    ///
    /// Returns Ok(Some(value)) if the variable exists and is a boolean,
    /// Ok(None) if the variable doesn't exist, or an error if it exists but isn't a boolean.
    pub fn get_global_bool_opt(&self, name: &str) -> LuaResult<Option<bool>> {
        match self.lua.globals().get::<LuaValue>(name) {
            Ok(LuaValue::Nil) => Ok(None),
            Ok(LuaValue::Boolean(b)) => Ok(Some(b)),
            Ok(_) => Err(LuaError::external(format!(
                "Global '{}' is not a boolean",
                name
            ))),
            Err(_) => Ok(None),
        }
    }

    /// Get an integer value from the Lua runtime globals.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the global variable
    ///
    /// # Returns
    ///
    /// Returns Ok(Some(value)) if the variable exists and is convertible to an integer,
    /// Ok(None) if the variable doesn't exist.
    pub fn get_global_int_opt(&self, name: &str) -> LuaResult<Option<i64>> {
        match self.lua.globals().get::<LuaValue>(name) {
            Ok(LuaValue::Nil) => Ok(None),
            Ok(LuaValue::Integer(i)) => Ok(Some(i)),
            Ok(LuaValue::Number(n)) => Ok(Some(n as i64)),
            Ok(_) => Err(LuaError::external(format!(
                "Global '{}' cannot be converted to integer",
                name
            ))),
            Err(_) => Ok(None),
        }
    }

     /// Get a table value from the Lua runtime globals.
     ///
     /// # Arguments
     ///
     /// * `name` - The name of the global variable
     ///
     /// # Returns
     ///
     /// Returns Ok(Some(table)) if the variable exists and is a table,
     /// Ok(None) if the variable doesn't exist, or an error if it exists but isn't a table.
     pub fn get_global_table_opt(&self, name: &str) -> LuaResult<Option<LuaTable>> {
         match self.lua.globals().get::<LuaValue>(name) {
             Ok(LuaValue::Nil) => Ok(None),
             Ok(LuaValue::Table(t)) => Ok(Some(t)),
             Ok(_) => Err(LuaError::external(format!(
                 "Global '{}' is not a table",
                 name
             ))),
             Err(_) => Ok(None),
         }
     }

     /// Iterate over all entries in a Lua table and call a closure for each entry.
     ///
     /// # Arguments
     ///
     /// * `table` - The Lua table to iterate
     /// * `f` - Closure that receives (key, value) for each entry
     ///
     /// # Returns
     ///
     /// Returns an error if iteration fails.
     pub fn iterate_table<F>(&self, table: &LuaTable, mut f: F) -> LuaResult<()>
     where
         F: FnMut(LuaValue, LuaValue) -> LuaResult<()>,
     {
         let pairs_fn = self.lua.globals().get::<LuaFunction>("pairs")?;
         let mut iter = pairs_fn.call::<LuaMultiValue>(table.clone())?;

         loop {
             let key_opt = iter.pop_front();
             let val_opt = iter.pop_front();

             match (key_opt, val_opt) {
                 (Some(key), Some(value)) => f(key, value)?,
                 (None, None) => break,
                 _ => break,
             }
         }

         Ok(())
     }

     /// Get a reference to the underlying Lua runtime for advanced use cases.
     ///
     /// This allows direct access to the mlua::Lua instance.
     pub fn inner(&self) -> &Lua {
         &self.lua
     }

     /// Extract all keybindings from the Lua globals.binds table.
     ///
     /// This method looks for a `binds` table in the Lua globals and extracts
     /// all keybinding entries. Each entry should have a key, action, and optional
     /// args field.
     ///
     /// # Returns
     ///
     /// Returns Ok(Vec of keybindings) or an error if extraction fails.
     pub fn get_keybindings(&self) -> LuaResult<Vec<(String, String, Vec<String>)>> {
         let mut keybindings = Vec::new();

         // Get the binds table
         match self.get_global_table_opt("binds")? {
             Some(binds_table) => {
                 // Iterate through each keybinding entry in the table
                 let mut index = 1i64;
                 loop {
                     let binding: LuaValue = binds_table.get(index)?;
                     if binding == LuaValue::Nil {
                         break;
                     }

                     if let Some(binding_table) = binding.as_table() {
                         // Extract key, action, and optional args
                         let key: String = binding_table
                             .get("key")
                             .unwrap_or_else(|_| "".to_string());
                         let action: String = binding_table
                             .get("action")
                             .unwrap_or_else(|_| "".to_string());

                          let args: Vec<String> = if let Ok(args_table) =
                              binding_table.get::<LuaTable>("args")
                          {
                             let mut args_vec = Vec::new();
                             let mut arg_index = 1i64;
                             loop {
                                 let arg: LuaValue = args_table.get(arg_index)?;
                                 if arg == LuaValue::Nil {
                                     break;
                                 }
                                 if let Some(arg_str) = arg.as_string() {
                                     args_vec.push(
                                         arg_str.to_string_lossy().to_string()
                                     );
                                 }
                                 arg_index += 1;
                             }
                             args_vec
                         } else {
                             Vec::new()
                         };

                         if !key.is_empty() && !action.is_empty() {
                             keybindings.push((key, action, args));
                         }
                     }

                     index += 1;
                 }
             }
             None => {
                 // No binds table found, return empty list
             }
         }

         Ok(keybindings)
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
