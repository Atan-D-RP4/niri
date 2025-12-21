//! Niri Lua module search paths and custom require implementation.
//!
//! Provides XDG-compliant search paths for user Lua modules and a custom
//! `require()` function that replaces Luau's built-in require (which is not
//! available in ALL_SAFE mode).
//!
//! # Search Paths
//!
//! - `$XDG_CONFIG_HOME/niri/lua/` (defaults to `~/.config/niri/lua/`)
//! - `$XDG_DATA_HOME/niri/lua/` (defaults to `~/.local/share/niri/lua/`)
//! - `/usr/share/niri/lua/`
//!
//! # Module Resolution
//!
//! - Relative paths (`./foo`, `../bar`) resolve from the calling file's directory
//! - Absolute paths (`foo`, `foo.bar`) search XDG paths in order
//! - Dot notation (`foo.bar`) converts to path separators (`foo/bar.lua`)
//! - The `init.lua` convention is supported (`foo/init.lua` for `require("foo")`)

use std::cell::RefCell;
use std::env;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use mlua::prelude::*;
use mlua::Compiler;

/// Get the default niri Lua search paths following XDG Base Directory spec.
///
/// Returns paths in priority order (user config first, system last).
pub fn default_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();

    // XDG_CONFIG_HOME or ~/.config (highest priority)
    let config_home = env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")));

    if let Some(config_dir) = config_home {
        paths.push(config_dir.join("niri/lua"));
    }

    // XDG_DATA_HOME or ~/.local/share
    let data_home = env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share")));

    if let Some(data_dir) = data_home {
        paths.push(data_dir.join("niri/lua"));
    }

    // System directory
    paths.push(PathBuf::from("/usr/share/niri/lua"));

    paths
}

/// Result of searching for a module.
#[derive(Debug)]
pub enum SearchResult {
    /// Module found at this path
    Found(PathBuf),
    /// Module not found, searched these paths
    NotFound(Vec<PathBuf>),
}

/// Resolve a module name to a file path by searching XDG paths.
///
/// Converts dot notation to path separators and tries both `.lua` files
/// and `init.lua` in directories.
pub fn resolve_module(module_name: &str, search_paths: &[PathBuf]) -> SearchResult {
    // Convert dot notation to path: "foo.bar" -> "foo/bar"
    let relative_path = module_name.replace('.', "/");

    let mut searched = Vec::new();

    for base_path in search_paths {
        // Try direct .lua file
        let candidate = base_path.join(format!("{}.lua", relative_path));
        searched.push(candidate.clone());
        if candidate.is_file() {
            return SearchResult::Found(candidate);
        }

        // Try init.lua in directory
        let candidate = base_path.join(&relative_path).join("init.lua");
        searched.push(candidate.clone());
        if candidate.is_file() {
            return SearchResult::Found(candidate);
        }
    }

    SearchResult::NotFound(searched)
}

/// Resolve a relative module path from a base directory.
///
/// Handles `./foo` and `../foo` style requires.
pub fn resolve_relative(base_dir: &Path, module_name: &str) -> SearchResult {
    // Strip the ./ or ../ prefix and handle the path
    let clean_name = module_name
        .trim_start_matches("./")
        .trim_start_matches("../");

    // Count how many parent directories to go up
    let parent_count = module_name.matches("../").count();

    let mut target_dir = base_dir.to_path_buf();
    for _ in 0..parent_count {
        target_dir = target_dir
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| target_dir.clone());
    }

    // If it starts with ./ (not ../), we stay in base_dir
    // The clean_name has the actual module path

    let mut searched = Vec::new();

    // Try direct .lua file
    let candidate = target_dir.join(format!("{}.lua", clean_name));
    searched.push(candidate.clone());
    if candidate.is_file() {
        return SearchResult::Found(candidate);
    }

    // Try init.lua in directory
    let candidate = target_dir.join(clean_name).join("init.lua");
    searched.push(candidate.clone());
    if candidate.is_file() {
        return SearchResult::Found(candidate);
    }

    SearchResult::NotFound(searched)
}

/// Format an error message for a module not found, listing all searched paths.
pub fn format_not_found_error(module_name: &str, searched: &[PathBuf]) -> String {
    let mut msg = format!("module '{}' not found:\n", module_name);
    for path in searched {
        msg.push_str(&format!("\tno file '{}'\n", path.display()));
    }
    msg
}

/// Register the custom `require()` function to the Lua runtime.
///
/// This replaces Luau's built-in require (which is unavailable in ALL_SAFE mode)
/// with a custom implementation that:
/// - Caches modules in `__niri_loaded`
/// - Supports relative requires (`./foo`, `../bar`) from the calling file
/// - Searches XDG paths for absolute requires (`foo`, `foo.bar`)
/// - Converts dot notation to path separators
///
/// # Arguments
///
/// * `lua` - The Lua runtime
/// * `compiler` - The Luau compiler for bytecode compilation
///
/// # Errors
///
/// Returns an error if the require function cannot be registered.
pub fn register_custom_require(lua: &Lua, compiler: Rc<RefCell<Compiler>>) -> LuaResult<()> {
    // Create module cache table
    lua.globals().set("__niri_loaded", lua.create_table()?)?;

    // Get search paths
    let search_paths = default_paths();

    // Create custom require function
    let require_fn = lua.create_function(move |lua, module_name: String| {
        // Check cache first
        let cache: LuaTable = lua.globals().get("__niri_loaded")?;
        let cached: LuaValue = cache.get(module_name.as_str())?;
        if cached != LuaValue::Nil {
            return Ok(cached);
        }

        // Get current file context for relative requires
        let current_file: Option<String> = lua.globals().get("__niri_current_file")?;

        // Resolve the module path
        let resolved = if module_name.starts_with("./") || module_name.starts_with("../") {
            // Relative require - resolve from current file's directory
            let current = current_file.ok_or_else(|| {
                LuaError::external(format!(
                    "relative require '{}' used without file context (are you in a REPL?)",
                    module_name
                ))
            })?;

            let base_dir = Path::new(&current)
                .parent()
                .ok_or_else(|| LuaError::external("cannot get parent directory of current file"))?;

            resolve_relative(base_dir, &module_name)
        } else {
            // Absolute require - search XDG paths
            resolve_module(&module_name, &search_paths)
        };

        // Handle resolution result
        let path = match resolved {
            SearchResult::Found(p) => p,
            SearchResult::NotFound(searched) => {
                return Err(LuaError::external(format_not_found_error(
                    &module_name,
                    &searched,
                )));
            }
        };

        // Save current file context for nested requires
        let old_current_file: LuaValue = lua.globals().get("__niri_current_file")?;

        // Get absolute path for the module
        let absolute_path = path.canonicalize().unwrap_or_else(|_| path.clone());
        let path_str = absolute_path.to_string_lossy().to_string();

        // Set new current file context
        lua.globals()
            .set("__niri_current_file", path_str.as_str())?;

        // Read and compile the module
        let code = std::fs::read_to_string(&path).map_err(|e| {
            LuaError::external(format!("failed to read module '{}': {}", path.display(), e))
        })?;

        let bytecode = compiler.borrow().compile(&code)?;

        // Execute with chunk name set for error messages
        let result: LuaValue = lua.load(bytecode).set_name(&path_str).eval()?;

        // Restore previous file context
        lua.globals().set("__niri_current_file", old_current_file)?;

        // If module returned nil, use true (Lua convention)
        let final_result = if result == LuaValue::Nil {
            LuaValue::Boolean(true)
        } else {
            result
        };

        // Cache the result
        cache.set(module_name.as_str(), final_result.clone())?;

        Ok(final_result)
    })?;

    lua.globals().set("require", require_fn)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_paths_not_empty() {
        let paths = default_paths();
        // Should have at least one path (system path always exists)
        assert!(!paths.is_empty());
        // System path is always last
        assert_eq!(paths.last().unwrap(), &PathBuf::from("/usr/share/niri/lua"));
    }

    #[test]
    fn default_paths_includes_system() {
        let paths = default_paths();
        assert!(paths.contains(&PathBuf::from("/usr/share/niri/lua")));
    }

    #[test]
    fn resolve_module_not_found() {
        let paths = vec![PathBuf::from("/nonexistent/path")];
        let result = resolve_module("nonexistent_module", &paths);
        match result {
            SearchResult::NotFound(searched) => {
                assert!(!searched.is_empty());
                // Should have tried both .lua and init.lua
                assert!(searched.len() >= 2);
            }
            SearchResult::Found(_) => panic!("Should not find nonexistent module"),
        }
    }

    #[test]
    fn resolve_module_dot_notation() {
        let paths = vec![PathBuf::from("/test")];
        let result = resolve_module("foo.bar.baz", &paths);
        match result {
            SearchResult::NotFound(searched) => {
                // Should convert dots to path separators
                assert!(searched
                    .iter()
                    .any(|p| p.to_string_lossy().contains("foo/bar/baz.lua")));
            }
            SearchResult::Found(_) => panic!("Should not find nonexistent module"),
        }
    }

    #[test]
    fn format_not_found_error_includes_paths() {
        let searched = vec![
            PathBuf::from("/a/b.lua"),
            PathBuf::from("/a/b/init.lua"),
            PathBuf::from("/c/d.lua"),
        ];
        let error = format_not_found_error("mymodule", &searched);

        assert!(error.contains("module 'mymodule' not found"));
        assert!(error.contains("/a/b.lua"));
        assert!(error.contains("/a/b/init.lua"));
        assert!(error.contains("/c/d.lua"));
    }

    #[test]
    fn resolve_relative_current_dir() {
        let base = PathBuf::from("/home/user/config");
        let result = resolve_relative(&base, "./utils");
        match result {
            SearchResult::NotFound(searched) => {
                assert!(searched
                    .iter()
                    .any(|p| p.to_string_lossy().contains("/home/user/config/utils.lua")));
            }
            SearchResult::Found(_) => panic!("Should not find nonexistent module"),
        }
    }

    #[test]
    fn resolve_relative_parent_dir() {
        let base = PathBuf::from("/home/user/config/subdir");
        let result = resolve_relative(&base, "../utils");
        match result {
            SearchResult::NotFound(searched) => {
                // Should go up one directory
                assert!(searched
                    .iter()
                    .any(|p| p.to_string_lossy().contains("/home/user/config/utils.lua")));
            }
            SearchResult::Found(_) => panic!("Should not find nonexistent module"),
        }
    }
}
