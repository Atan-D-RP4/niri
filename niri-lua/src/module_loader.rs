//! Custom Lua module loader with support for plugins and multiple search paths.
//!
//! This module implements a custom `require` function that extends Lua's standard
//! module loading with support for Niri plugins in custom directories.
//!
//! # Search Paths
//!
//! Modules are searched in this order:
//! 1. User plugins: `~/.config/niri/plugins/`
//! 2. System plugins: `/usr/local/share/niri/plugins/`
//! 3. Vendor plugins: `/usr/share/niri/plugins/`
//! 4. Standard Lua paths
//!
//! # Module Resolution
//!
//! For `require "foo.bar"`:
//! - Tries `foo/bar.lua`
//! - Tries `foo/bar/init.lua`
//!
//! # Example
//!
//! ```lua
//! local helpers = require "helpers"           -- Load helpers.lua from search path
//! local plugin = require "plugins.myplugin"   -- Load plugins/myplugin.lua
//! ```

use std::fs;
use std::path::PathBuf;

use log::{debug, error, warn};
use mlua::prelude::*;

/// Module loader for custom search paths.
#[derive(Debug, Clone)]
pub struct ModuleLoader {
    search_paths: Vec<PathBuf>,
}

impl ModuleLoader {
    /// Create a new module loader with default search paths.
    pub fn new() -> Self {
        let mut search_paths = vec![];

        // User plugins directory
        if let Ok(home) = std::env::var("HOME") {
            search_paths.push(PathBuf::from(&home).join(".config/niri/plugins"));
            search_paths.push(PathBuf::from(&home).join(".local/share/niri/plugins"));
        }

        // System plugins
        search_paths.push(PathBuf::from("/usr/local/share/niri/plugins"));
        search_paths.push(PathBuf::from("/usr/share/niri/plugins"));

        // Standard Lua paths (in current directory and system dirs)
        search_paths.push(PathBuf::from("."));

        Self { search_paths }
    }

    /// Create a module loader with custom search paths.
    pub fn with_paths(paths: Vec<PathBuf>) -> Self {
        let mut loader = Self::new();
        loader.search_paths = paths;
        loader
    }

    /// Add a search path to the loader.
    pub fn add_search_path(&mut self, path: PathBuf) {
        self.search_paths.insert(0, path);
    }

    /// Find a module file in the search paths.
    ///
    /// For module name `foo.bar`, tries:
    /// - `foo/bar.lua`
    /// - `foo/bar/init.lua`
    fn find_module(&self, module_name: &str) -> Option<PathBuf> {
        let path_parts: Vec<&str> = module_name.split('.').collect();
        let relative_path = path_parts.join("/");

        for search_path in &self.search_paths {
            // Try module.lua
            let file_path = search_path.join(format!("{}.lua", relative_path));
            if file_path.exists() && file_path.is_file() {
                debug!("Found module {} at {}", module_name, file_path.display());
                return Some(file_path);
            }

            // Try module/init.lua
            let init_path = search_path.join(&relative_path).join("init.lua");
            if init_path.exists() && init_path.is_file() {
                debug!("Found module {} at {}", module_name, init_path.display());
                return Some(init_path);
            }
        }

        warn!("Module not found: {}", module_name);
        None
    }

    /// Load a module by name.
    pub fn load_module(&self, lua: &Lua, module_name: &str) -> LuaResult<LuaValue> {
        match self.find_module(module_name) {
            Some(path) => {
                debug!("Loading module {} from {}", module_name, path.display());
                match fs::read_to_string(&path) {
                    Ok(source) => {
                        // Execute module code
                        lua.load(&source).set_name(module_name).eval()
                    }
                    Err(e) => {
                        error!("Failed to read module {}: {}", module_name, e);
                        Err(LuaError::RuntimeError(format!(
                            "Failed to read module {}: {}",
                            module_name, e
                        )))
                    }
                }
            }
            None => {
                error!("Module not found: {}", module_name);
                Err(LuaError::RuntimeError(format!(
                    "module '{}' not found in search paths",
                    module_name
                )))
            }
        }
    }

    /// Register the custom require function to Lua.
    pub fn register_to_lua(&self, lua: &Lua) -> LuaResult<()> {
        let loader = self.clone();

        // Override the global `require` function
        let require = lua.create_function(move |lua, module_name: String| {
            loader.load_module(lua, &module_name)
        })?;

        lua.globals().set("require", require)?;

        debug!(
            "Registered custom module loader with {} search paths",
            self.search_paths.len()
        );
        for (i, path) in self.search_paths.iter().enumerate() {
            debug!("  [{}] {}", i + 1, path.display());
        }

        Ok(())
    }
}

impl Default for ModuleLoader {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use std::fs::File;
    use std::io::Write;

    use tempfile::TempDir;

    use super::*;

    #[test]
    fn module_loader_creation() {
        let loader = ModuleLoader::new();
        assert!(!loader.search_paths.is_empty());
    }

    #[test]
    fn find_module_lua_file() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path().to_path_buf();

        // Create a test module
        let module_path = temp_path.join("test_module.lua");
        let mut file = File::create(&module_path).unwrap();
        file.write_all(b"return { value = 42 }").unwrap();

        let loader = ModuleLoader::with_paths(vec![temp_path]);
        let found = loader.find_module("test_module");

        assert!(found.is_some());
        assert_eq!(found.unwrap(), module_path);
    }

    #[test]
    fn find_module_init_file() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path().to_path_buf();

        // Create a test package
        let package_dir = temp_path.join("test_pkg");
        fs::create_dir(&package_dir).unwrap();
        let init_path = package_dir.join("init.lua");
        let mut file = File::create(&init_path).unwrap();
        file.write_all(b"return { name = 'test' }").unwrap();

        let loader = ModuleLoader::with_paths(vec![temp_path]);
        let found = loader.find_module("test_pkg");

        assert!(found.is_some());
        assert_eq!(found.unwrap(), init_path);
    }

    #[test]
    fn find_module_not_found() {
        let temp_dir = TempDir::new().unwrap();
        let loader = ModuleLoader::with_paths(vec![temp_dir.path().to_path_buf()]);
        let found = loader.find_module("nonexistent");

        assert!(found.is_none());
    }

    #[test]
    fn load_module_lua() {
        let lua = Lua::new();
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path().to_path_buf();

        // Create a test module
        let module_path = temp_path.join("test.lua");
        let mut file = File::create(&module_path).unwrap();
        file.write_all(b"return { value = 42 }").unwrap();

        let loader = ModuleLoader::with_paths(vec![temp_path]);
        let _result: LuaTable = lua.create_table().unwrap();
        if let LuaValue::Table(t) = loader.load_module(&lua, "test").unwrap() {
            let value: i32 = t.get("value").unwrap();
            assert_eq!(value, 42);
        } else {
            panic!("Expected table result");
        }
    }

    #[test]
    fn register_custom_require() {
        let lua = Lua::new();
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path().to_path_buf();

        // Create a test module
        let module_path = temp_path.join("mymod.lua");
        let mut file = File::create(&module_path).unwrap();
        file.write_all(b"return { msg = 'hello' }").unwrap();

        let loader = ModuleLoader::with_paths(vec![temp_path]);
        loader.register_to_lua(&lua).unwrap();

        // Test using the custom require
        let result: String = lua
            .load("local m = require 'mymod'; return m.msg")
            .eval()
            .unwrap();

        assert_eq!(result, "hello");
    }

    #[test]
    fn module_not_found_error() {
        let lua = Lua::new();
        let temp_dir = TempDir::new().unwrap();

        let loader = ModuleLoader::with_paths(vec![temp_dir.path().to_path_buf()]);
        loader.register_to_lua(&lua).unwrap();

        // Try to load non-existent module
        let result = lua.load("require 'nonexistent'").eval::<()>();

        assert!(result.is_err());
    }
}
