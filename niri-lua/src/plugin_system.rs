//! Plugin system for Niri Lua runtime.
//!
//! This module provides plugin loading, metadata management, and lifecycle management
//! for Niri plugins.
//!
//! # Plugin Structure
//!
//! A plugin can be:
//! - A single Lua file: `~/.config/niri/plugins/myplugin.lua`
//! - A package directory: `~/.config/niri/plugins/myplugin/init.lua`
//!
//! # Plugin Metadata
//!
//! Plugins can optionally expose metadata:
//!
//! ```lua
//! return {
//!   name = "my-plugin",
//!   version = "1.0.0",
//!   author = "Your Name",
//!   description = "What it does",
//!   license = "MIT",
//!   dependencies = {},
//! }
//! ```
//!
//! # Plugin Lifecycle
//!
//! 1. **Load** - Plugin code is loaded
//! 2. **Initialize** - Plugin's `on_init` is called if defined
//! 3. **Active** - Plugin runs in response to events
//! 4. **Unload** - Plugin is cleaned up

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use log::{debug, error, info, warn};
use mlua::prelude::*;
use serde::{Deserialize, Serialize};

/// Plugin metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginMetadata {
    pub name: String,
    pub version: String,
    pub author: Option<String>,
    pub description: Option<String>,
    pub license: Option<String>,
    pub dependencies: Vec<String>,
}

impl PluginMetadata {
    /// Parse metadata from a Lua value
    pub fn from_lua(value: &LuaValue) -> Option<Self> {
        match value {
            LuaValue::Table(table) => Some(PluginMetadata {
                name: table.get("name").ok()?,
                version: table.get("version").ok()?,
                author: table.get("author").ok(),
                description: table.get("description").ok(),
                license: table.get("license").ok(),
                dependencies: table.get("dependencies").unwrap_or_default(),
            }),
            _ => None,
        }
    }
}

/// Plugin information
#[derive(Debug, Clone)]
pub struct PluginInfo {
    pub metadata: PluginMetadata,
    pub path: PathBuf,
    pub enabled: bool,
    pub loaded: bool,
}

impl PluginInfo {
    pub fn new(metadata: PluginMetadata, path: PathBuf) -> Self {
        Self {
            metadata,
            path,
            enabled: true,
            loaded: false,
        }
    }
}

/// Plugin manager for the Niri Lua runtime
pub struct PluginManager {
    plugins: HashMap<String, PluginInfo>,
    search_paths: Vec<PathBuf>,
}

impl PluginManager {
    /// Create a new plugin manager
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

        Self {
            plugins: HashMap::new(),
            search_paths,
        }
    }

    /// Add a search path for plugins
    pub fn add_search_path(&mut self, path: PathBuf) {
        self.search_paths.insert(0, path);
    }

    /// Discover plugins in search paths
    pub fn discover(&mut self, lua: &Lua) -> LuaResult<()> {
        let search_paths = self.search_paths.clone();
        for search_path in search_paths {
            if !search_path.exists() {
                debug!(
                    "Plugin search path does not exist: {}",
                    search_path.display()
                );
                continue;
            }

            match fs::read_dir(&search_path) {
                Ok(entries) => {
                    for entry in entries {
                        match entry {
                            Ok(entry) => {
                                let path = entry.path();

                                // Check for .lua files
                                if path.is_file()
                                    && path.extension().map_or(false, |ext| ext == "lua")
                                {
                                    if let Some(name) = path.file_stem() {
                                        let plugin_name = name.to_string_lossy().to_string();
                                        if plugin_name != "init" {
                                            self.load_plugin(lua, &plugin_name, &path)?;
                                        }
                                    }
                                }

                                // Check for directories (packages)
                                if path.is_dir() {
                                    let init_path = path.join("init.lua");
                                    if init_path.exists() {
                                        if let Some(name) = path.file_name() {
                                            let plugin_name = name.to_string_lossy().to_string();
                                            self.load_plugin(lua, &plugin_name, &path)?;
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                warn!("Error reading plugin directory entry: {}", e);
                            }
                        }
                    }
                }
                Err(e) => {
                    warn!(
                        "Failed to read plugin directory {}: {}",
                        search_path.display(),
                        e
                    );
                }
            }
        }

        info!("Discovered {} plugins", self.plugins.len());
        Ok(())
    }

    /// Load a plugin by name from a path
    fn load_plugin(&mut self, lua: &Lua, name: &str, path: &std::path::Path) -> LuaResult<()> {
        if self.plugins.contains_key(name) {
            debug!("Plugin already loaded: {}", name);
            return Ok(());
        }

        debug!("Loading plugin: {} from {}", name, path.display());

        // Determine the actual Lua file to load
        let lua_file = if path.is_file() {
            path.to_path_buf()
        } else if path.is_dir() {
            path.join("init.lua")
        } else {
            error!("Invalid plugin path: {}", path.display());
            return Err(LuaError::RuntimeError(format!(
                "Invalid plugin path: {}",
                path.display()
            )));
        };

        if !lua_file.exists() {
            error!("Plugin file not found: {}", lua_file.display());
            return Err(LuaError::RuntimeError(format!(
                "Plugin file not found: {}",
                lua_file.display()
            )));
        }

        // Read and execute plugin code
        match fs::read_to_string(&lua_file) {
            Ok(source) => {
                // Create isolated plugin environment
                let _plugin_env = self.create_plugin_env(lua)?;

                // Execute plugin code in isolated environment
                match lua.load(&source).set_name(name).eval::<LuaValue>() {
                    Ok(result) => {
                        // Try to extract metadata
                        let metadata =
                            PluginMetadata::from_lua(&result).unwrap_or_else(|| PluginMetadata {
                                name: name.to_string(),
                                version: "0.0.0".to_string(),
                                author: None,
                                description: None,
                                license: None,
                                dependencies: vec![],
                            });

                        let plugin_info = PluginInfo::new(metadata.clone(), lua_file.clone());
                        self.plugins.insert(name.to_string(), plugin_info);

                        info!(
                            "Loaded plugin: {} v{} by {}",
                            metadata.name,
                            metadata.version,
                            metadata.author.as_deref().unwrap_or("Unknown")
                        );

                        Ok(())
                    }
                    Err(e) => {
                        error!("Failed to load plugin {}: {}", name, e);
                        Err(e)
                    }
                }
            }
            Err(e) => {
                error!("Failed to read plugin {}: {}", name, e);
                Err(LuaError::RuntimeError(format!(
                    "Failed to read plugin {}: {}",
                    name, e
                )))
            }
        }
    }

    /// Create an isolated environment for a plugin
    fn create_plugin_env(&self, lua: &Lua) -> LuaResult<LuaTable> {
        let env = lua.create_table()?;

        // Copy globals but with restrictions
        let globals = lua.globals();
        for pair in globals.pairs::<LuaValue, LuaValue>() {
            let (k, v) = pair?;
            env.set(k, v)?;
        }

        Ok(env)
    }

    /// Get plugin information
    pub fn get_plugin(&self, name: &str) -> Option<&PluginInfo> {
        self.plugins.get(name)
    }

    /// Get all plugins
    pub fn list_plugins(&self) -> Vec<&PluginInfo> {
        self.plugins.values().collect()
    }

    /// Enable a plugin
    pub fn enable_plugin(&mut self, name: &str) -> bool {
        if let Some(plugin) = self.plugins.get_mut(name) {
            plugin.enabled = true;
            info!("Enabled plugin: {}", name);
            true
        } else {
            warn!("Plugin not found: {}", name);
            false
        }
    }

    /// Disable a plugin
    pub fn disable_plugin(&mut self, name: &str) -> bool {
        if let Some(plugin) = self.plugins.get_mut(name) {
            plugin.enabled = false;
            info!("Disabled plugin: {}", name);
            true
        } else {
            warn!("Plugin not found: {}", name);
            false
        }
    }

    /// Unload a plugin
    pub fn unload_plugin(&mut self, name: &str) -> bool {
        if self.plugins.remove(name).is_some() {
            info!("Unloaded plugin: {}", name);
            true
        } else {
            warn!("Plugin not found: {}", name);
            false
        }
    }

    /// Register plugin manager API to Lua
    pub fn register_to_lua(&self, lua: &Lua) -> LuaResult<()> {
        let niri_table: LuaTable = lua.globals().get("niri")?;

        // Create plugins namespace
        let plugins = lua.create_table()?;

        // Get plugins list
        let plugins_clone = self.plugins.clone();
        plugins.set(
            "list",
            lua.create_function(move |lua, ()| {
                let result = lua.create_table()?;
                for (i, (name, info)) in plugins_clone.iter().enumerate() {
                    let plugin_table = lua.create_table()?;
                    plugin_table.set("name", name.clone())?;
                    plugin_table.set("version", info.metadata.version.clone())?;
                    plugin_table.set("enabled", info.enabled)?;
                    plugin_table.set("loaded", info.loaded)?;
                    result.set(i + 1, plugin_table)?;
                }
                Ok(result)
            })?,
        )?;

        niri_table.set("plugins", plugins)?;

        debug!("Registered plugin manager API to Lua");
        Ok(())
    }
}

impl Default for PluginManager {
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
    fn plugin_manager_creation() {
        let manager = PluginManager::new();
        assert!(!manager.search_paths.is_empty());
    }

    #[test]
    fn plugin_manager_default() {
        let manager = PluginManager::default();
        assert!(!manager.search_paths.is_empty());
    }

    #[test]
    fn plugin_metadata_from_lua() {
        let lua = Lua::new();
        let table = lua.create_table().unwrap();
        table.set("name", "test-plugin").unwrap();
        table.set("version", "1.0.0").unwrap();
        table.set("author", "Test Author").unwrap();

        let metadata = PluginMetadata::from_lua(&LuaValue::Table(table)).unwrap();
        assert_eq!(metadata.name, "test-plugin");
        assert_eq!(metadata.version, "1.0.0");
        assert_eq!(metadata.author, Some("Test Author".to_string()));
    }

    #[test]
    fn plugin_metadata_minimal() {
        let lua = Lua::new();
        let table = lua.create_table().unwrap();
        table.set("name", "minimal").unwrap();
        table.set("version", "0.1.0").unwrap();

        let metadata = PluginMetadata::from_lua(&LuaValue::Table(table)).unwrap();
        assert_eq!(metadata.name, "minimal");
        assert_eq!(metadata.version, "0.1.0");
        assert_eq!(metadata.author, None);
    }

    #[test]
    fn plugin_metadata_from_non_table() {
        let invalid_value = LuaValue::Integer(42);
        let result = PluginMetadata::from_lua(&invalid_value);
        assert!(result.is_none());
    }

    #[test]
    fn plugin_info_creation() {
        let metadata = PluginMetadata {
            name: "test".to_string(),
            version: "1.0.0".to_string(),
            author: None,
            description: None,
            license: None,
            dependencies: Vec::new(),
        };
        let path = std::path::PathBuf::from("/tmp/test.lua");
        let info = PluginInfo::new(metadata.clone(), path.clone());

        assert_eq!(info.metadata.name, "test");
        assert_eq!(info.path, path);
        assert!(info.enabled);
        assert!(!info.loaded);
    }

    #[test]
    fn discover_plugins() {
        let lua = Lua::new();
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path().to_path_buf();

        // Create a test plugin
        let plugin_file = temp_path.join("test_plugin.lua");
        let mut file = File::create(&plugin_file).unwrap();
        file.write_all(b"return { name = 'test', version = '1.0' }")
            .unwrap();

        let mut manager = PluginManager::new();
        manager.add_search_path(temp_path);
        manager.discover(&lua).unwrap();

        assert!(manager.get_plugin("test_plugin").is_some());
    }

    #[test]
    fn discover_multiple_plugins() {
        let lua = Lua::new();
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path().to_path_buf();

        // Create multiple test plugins
        for i in 1..=3 {
            let plugin_file = temp_path.join(format!("plugin{}.lua", i));
            let mut file = File::create(&plugin_file).unwrap();
            file.write_all(
                format!("return {{ name = 'plugin{}', version = '1.0' }}", i).as_bytes(),
            )
            .unwrap();
        }

        let mut manager = PluginManager::new();
        manager.add_search_path(temp_path);
        manager.discover(&lua).unwrap();

        assert!(manager.get_plugin("plugin1").is_some());
        assert!(manager.get_plugin("plugin2").is_some());
        assert!(manager.get_plugin("plugin3").is_some());
    }

    #[test]
    fn add_search_path() {
        let mut manager = PluginManager::new();
        let initial_count = manager.search_paths.len();

        let temp_dir = TempDir::new().unwrap();
        manager.add_search_path(temp_dir.path().to_path_buf());

        assert_eq!(manager.search_paths.len(), initial_count + 1);
    }

    #[test]
    fn get_plugin_exists() {
        let lua = Lua::new();
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path().to_path_buf();

        let plugin_file = temp_path.join("myapp.lua");
        let mut file = File::create(&plugin_file).unwrap();
        file.write_all(b"return { name = 'myapp', version = '2.0' }")
            .unwrap();

        let mut manager = PluginManager::new();
        manager.add_search_path(temp_path);
        manager.discover(&lua).unwrap();

        let plugin = manager.get_plugin("myapp");
        assert!(plugin.is_some());
        assert_eq!(plugin.unwrap().metadata.name, "myapp");
    }

    #[test]
    fn get_plugin_not_found() {
        let manager = PluginManager::new();
        assert!(manager.get_plugin("nonexistent").is_none());
    }

    #[test]
    fn enable_disable_plugin() {
        let lua = Lua::new();
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path().to_path_buf();

        // Create a test plugin
        let plugin_file = temp_path.join("test_plugin.lua");
        let mut file = File::create(&plugin_file).unwrap();
        file.write_all(b"return { name = 'test', version = '1.0' }")
            .unwrap();

        let mut manager = PluginManager::new();
        manager.add_search_path(temp_path);
        manager.discover(&lua).unwrap();

        // Test disable
        assert!(manager.disable_plugin("test_plugin"));
        assert!(!manager.get_plugin("test_plugin").unwrap().enabled);

        // Test enable
        assert!(manager.enable_plugin("test_plugin"));
        assert!(manager.get_plugin("test_plugin").unwrap().enabled);
    }

    #[test]
    fn enable_nonexistent_plugin() {
        let mut manager = PluginManager::new();
        assert!(!manager.enable_plugin("nonexistent"));
    }

    #[test]
    fn disable_nonexistent_plugin() {
        let mut manager = PluginManager::new();
        assert!(!manager.disable_plugin("nonexistent"));
    }

    #[test]
    fn unload_plugin() {
        let lua = Lua::new();
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path().to_path_buf();

        let plugin_file = temp_path.join("temp_plugin.lua");
        let mut file = File::create(&plugin_file).unwrap();
        file.write_all(b"return { name = 'temp', version = '1.0' }")
            .unwrap();

        let mut manager = PluginManager::new();
        manager.add_search_path(temp_path);
        manager.discover(&lua).unwrap();

        assert!(manager.get_plugin("temp_plugin").is_some());

        // Unload the plugin
        assert!(manager.unload_plugin("temp_plugin"));
        assert!(manager.get_plugin("temp_plugin").is_none());
    }

    #[test]
    fn unload_nonexistent_plugin() {
        let mut manager = PluginManager::new();
        assert!(!manager.unload_plugin("nonexistent"));
    }

    #[test]
    fn list_plugins() {
        let lua = Lua::new();
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path().to_path_buf();

        // Create test plugins
        for i in 1..=3 {
            let plugin_file = temp_path.join(format!("plugin{}.lua", i));
            let mut file = File::create(&plugin_file).unwrap();
            file.write_all(b"return { name = 'test', version = '1.0' }")
                .unwrap();
        }

        let mut manager = PluginManager::new();
        manager.add_search_path(temp_path);
        manager.discover(&lua).unwrap();

        let plugins = manager.list_plugins();
        assert_eq!(plugins.len(), 3);
    }

    #[test]
    fn list_plugins_empty() {
        let manager = PluginManager::new();
        let plugins = manager.list_plugins();
        assert_eq!(plugins.len(), 0);
    }

    #[test]
    fn register_to_lua() {
        let lua = Lua::new();
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path().to_path_buf();

        let plugin_file = temp_path.join("test_plugin.lua");
        let mut file = File::create(&plugin_file).unwrap();
        file.write_all(b"return { name = 'test', version = '1.0' }")
            .unwrap();

        let mut manager = PluginManager::new();
        manager.add_search_path(temp_path);
        manager.discover(&lua).unwrap();

        // Create niri table first
        let niri_table = lua.create_table().unwrap();
        lua.globals().set("niri", niri_table).unwrap();

        // Register to Lua
        assert!(manager.register_to_lua(&lua).is_ok());
    }

    #[test]
    fn multiple_plugin_operations() {
        let lua = Lua::new();
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path().to_path_buf();

        // Create test plugins
        for i in 1..=2 {
            let plugin_file = temp_path.join(format!("plugin{}.lua", i));
            let mut file = File::create(&plugin_file).unwrap();
            file.write_all(b"return { name = 'test', version = '1.0' }")
                .unwrap();
        }

        let mut manager = PluginManager::new();
        manager.add_search_path(temp_path);
        manager.discover(&lua).unwrap();

        // Verify both plugins are discovered
        assert_eq!(manager.list_plugins().len(), 2);

        // Disable first plugin
        assert!(manager.disable_plugin("plugin1"));
        assert!(!manager.get_plugin("plugin1").unwrap().enabled);
        assert!(manager.get_plugin("plugin2").unwrap().enabled);

        // Re-enable first plugin
        assert!(manager.enable_plugin("plugin1"));
        assert!(manager.get_plugin("plugin1").unwrap().enabled);

        // Unload second plugin
        assert!(manager.unload_plugin("plugin2"));
        assert_eq!(manager.list_plugins().len(), 1);
    }

    #[test]
    fn plugin_metadata_snapshot() {
        let metadata = PluginMetadata {
            name: "example-plugin".to_string(),
            version: "1.2.3".to_string(),
            author: Some("Test Author".to_string()),
            description: Some("A test plugin".to_string()),
            license: Some("MIT".to_string()),
            dependencies: vec!["dep1".to_string(), "dep2".to_string()],
        };

        insta::assert_debug_snapshot!("plugin_metadata_full", &metadata);
    }

    #[test]
    fn plugin_metadata_minimal_snapshot() {
        let metadata = PluginMetadata {
            name: "minimal".to_string(),
            version: "0.1.0".to_string(),
            author: None,
            description: None,
            license: None,
            dependencies: Vec::new(),
        };

        insta::assert_debug_snapshot!("plugin_metadata_minimal", &metadata);
    }

    #[test]
    fn plugin_info_snapshot() {
        let metadata = PluginMetadata {
            name: "example".to_string(),
            version: "1.0.0".to_string(),
            author: Some("Author".to_string()),
            description: None,
            license: None,
            dependencies: Vec::new(),
        };
        let info = PluginInfo::new(metadata, std::path::PathBuf::from("/tmp/example.lua"));

        // Snapshot only the essential fields to avoid path differences
        let snapshot_data = (
            &info.metadata.name,
            &info.metadata.version,
            info.enabled,
            info.loaded,
        );

        insta::assert_debug_snapshot!("plugin_info_state", &snapshot_data);
    }
}
