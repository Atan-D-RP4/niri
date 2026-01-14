//! Niri Luau module search paths and custom require implementation.
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
use std::path::{Path, PathBuf};
use std::rc::Rc;

use dirs;
use mlua::prelude::*;
use mlua::Compiler;

/// Get the default niri Lua search paths following XDG Base Directory spec.
///
/// Returns paths in priority order (user config first, system last).
pub fn lua_module_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();

    // XDG_CONFIG_HOME or ~/.config (highest priority)
    if let Some(config_dir) = dirs::config_dir() {
        roots.push(config_dir.join("niri/lua"));
    }

    // XDG_DATA_HOME or ~/.local/share
    if let Some(data_dir) = dirs::data_dir() {
        roots.push(data_dir.join("niri/lua"));
    }

    // System directory
    roots.push(PathBuf::from("/usr/share/niri/lua"));

    roots
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
fn resolve_module(name: &str, roots: &[PathBuf]) -> SearchResult {
    // Convert dot notation to path: "foo.bar" -> "foo/bar"
    let relative_path = name.replace('.', "/");

    let mut searched = Vec::new();

    for base_path in roots {
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
/// Handles `./foo`, `../foo`, and multi-segment paths like `../shared/utils`.
fn resolve_relative(base_dir: &Path, module_name: &str) -> SearchResult {
    // Parse the module name into path segments
    // Handle leading ./, ../ prefixes specially
    let (up_count, remaining) = if let Some(stripped) = module_name.strip_prefix("./") {
        (0, stripped)
    } else if module_name.starts_with("../") {
        // Count only leading ../ prefixes and strip them
        let mut count = 0;
        let mut rest = module_name;
        while let Some(stripped) = rest.strip_prefix("../") {
            count += 1;
            rest = stripped;
        }
        (count, rest)
    } else {
        (0, module_name)
    };

    // Walk up the base directory
    let mut target_dir = base_dir.to_path_buf();
    for _ in 0..up_count {
        target_dir = target_dir
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| target_dir.clone());
    }

    // If remaining is empty (e.g., just "../"), we've walked up but need to search in parent
    if remaining.is_empty() {
        return SearchResult::NotFound(vec![target_dir.join("init.lua")]);
    }

    let mut searched = Vec::new();

    // Try direct .lua file
    let candidate = target_dir.join(format!("{}.lua", remaining));
    searched.push(candidate.clone());
    if candidate.is_file() {
        return SearchResult::Found(candidate);
    }

    // Try init.lua in directory
    let candidate = target_dir.join(remaining).join("init.lua");
    searched.push(candidate.clone());
    if candidate.is_file() {
        return SearchResult::Found(candidate);
    }

    SearchResult::NotFound(searched)
}

/// Format an error message for a module not found, listing all searched paths.
fn format_not_found_error(module_name: &str, searched: &[PathBuf]) -> String {
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

    // Get search roots
    let roots = lua_module_roots();

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
            resolve_module(&module_name, &roots)
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

    #[test]
    fn resolve_relative_nested_path() {
        // Test multi-segment relative paths like ../shared/utils
        let base = PathBuf::from("/home/user/config/subdir");
        let result = resolve_relative(&base, "../shared/utils");
        match result {
            SearchResult::NotFound(searched) => {
                // Should go up one dir to /home/user/config, then resolve shared/utils
                assert!(searched.iter().any(|p| p
                    .to_string_lossy()
                    .contains("/home/user/config/shared/utils.lua")));
            }
            SearchResult::Found(_) => panic!("Should not find nonexistent module"),
        }
    }
}
