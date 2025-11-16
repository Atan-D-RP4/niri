# Tier 5 Specification: Plugin Ecosystem & Lifecycle

**Duration:** Weeks 9-10  
**Estimated LOC:** 250-300 Rust + 200 documentation  
**Complexity:** Very High (Complete plugin manager required)

## Overview

Tier 5 completes the plugin ecosystem with **full lifecycle management, dependency resolution, and distribution infrastructure**. Users can:
- Install, enable, disable, and uninstall plugins
- Manage plugin dependencies automatically
- Persist plugin state across reloads
- Share and discover plugins
- Build production-grade plugins

This tier is critical because it enables a thriving plugin ecosystem around Niri.

---

## Architecture

```
Plugin Ecosystem:
  - Plugin Registry: Central index of all installed plugins
  - Plugin Manager: Lifecycle orchestration
  - Plugin Sandbox: Environment isolation for each plugin
  - Dependency Resolver: Handles plugin dependencies
  - State Persistence: JSON/TOML storage per plugin
  - Plugin Commands: IPC interface for management
  
Directory Structure:
  ~/.config/niri/plugins/              # User-installed plugins
  ~/.local/share/niri/plugins/         # Plugin state/cache
  
Plugin Metadata (expanded):
  name, version, author, license
  dependencies, permissions
  entry_point, assets, documentation
  compatibility info
```

---

## Detailed Specifications

### 1. Plugin Manager (`src/lua_extensions/plugin_manager.rs`)

#### Purpose
Orchestrate plugin lifecycle and state management.

#### Core Structures

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginMetadata {
    pub name: String,
    pub version: String,
    pub author: Option<String>,
    pub description: Option<String>,
    pub license: Option<String>,
    pub repository: Option<String>,
    pub entry_point: Option<String>,  // Default: init.lua
    pub dependencies: HashMap<String, VersionConstraint>,
    pub permissions: PluginPermissions,
    pub compatibility: PluginCompatibility,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionConstraint {
    pub min: Option<String>,
    pub max: Option<String>,
    pub exact: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct PluginPermissions {
    pub access_state: bool,
    pub access_input: bool,
    pub modify_config: bool,
    pub spawn_processes: bool,
    pub filesystem_read: bool,
    pub filesystem_write: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginCompatibility {
    pub min_niri_version: String,
    pub max_niri_version: Option<String>,
    pub lua_version: String,  // "5.2" with LuaJIT
}

#[derive(Debug)]
pub struct PluginRegistry {
    plugins: HashMap<String, PluginInfo>,
    enabled: HashSet<String>,
    disabled: HashSet<String>,
}

#[derive(Debug)]
pub struct PluginInfo {
    pub path: PathBuf,
    pub metadata: PluginMetadata,
    pub state_dir: PathBuf,
    pub loaded: bool,
}

pub struct PluginManager {
    registry: PluginRegistry,
    runtime: Arc<LuaRuntime>,
    state_dir: PathBuf,
}

impl PluginManager {
    /// Create new plugin manager
    pub fn new(runtime: Arc<LuaRuntime>, state_dir: PathBuf) -> anyhow::Result<Self> {
        let mut manager = PluginManager {
            registry: PluginRegistry::new(),
            runtime,
            state_dir,
        };
        
        // Discover plugins on creation
        manager.discover_plugins()?;
        Ok(manager)
    }
    
    /// Discover all plugins in plugin directory
    pub fn discover_plugins(&mut self) -> anyhow::Result<()> {
        let plugin_dir = dirs::config_dir()
            .unwrap()
            .join("niri/plugins");
        
        if !plugin_dir.exists() {
            std::fs::create_dir_all(&plugin_dir)?;
            return Ok(());
        }
        
        for entry in std::fs::read_dir(&plugin_dir)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.is_dir() {
                if let Ok(metadata) = Self::load_plugin_metadata(&path) {
                    let state_dir = self.state_dir.join(&metadata.name);
                    std::fs::create_dir_all(&state_dir)?;
                    
                    let plugin_info = PluginInfo {
                        path,
                        metadata,
                        state_dir,
                        loaded: false,
                    };
                    
                    self.registry.add_plugin(plugin_info);
                }
            }
        }
        
        Ok(())
    }
    
    /// Load plugin metadata from init.toml or init.json
    fn load_plugin_metadata(plugin_dir: &Path) -> anyhow::Result<PluginMetadata> {
        // Try init.toml first
        let toml_path = plugin_dir.join("init.toml");
        if toml_path.exists() {
            let content = std::fs::read_to_string(toml_path)?;
            return Ok(toml::from_str(&content)?);
        }
        
        // Try init.json
        let json_path = plugin_dir.join("init.json");
        if json_path.exists() {
            let content = std::fs::read_to_string(json_path)?;
            return Ok(serde_json::from_str(&content)?);
        }
        
        // Generate default metadata
        Ok(PluginMetadata {
            name: plugin_dir
                .file_name()
                .unwrap()
                .to_string_lossy()
                .to_string(),
            version: "0.1.0".to_string(),
            author: None,
            description: None,
            license: None,
            repository: None,
            entry_point: Some("init.lua".to_string()),
            dependencies: HashMap::new(),
            permissions: PluginPermissions::default(),
            compatibility: PluginCompatibility {
                min_niri_version: env!("CARGO_PKG_VERSION").to_string(),
                max_niri_version: None,
                lua_version: "5.2".to_string(),
            },
        })
    }
    
    /// Enable plugin
    pub fn enable_plugin(&mut self, name: &str) -> anyhow::Result<()> {
        self.registry.validate_plugin_exists(name)?;
        
        // Check compatibility
        self.registry.check_compatibility(name)?;
        
        // Load plugin via Lua
        self.load_plugin(name)?;
        
        // Mark as enabled
        self.registry.enable(name);
        
        Ok(())
    }
    
    /// Disable plugin
    pub fn disable_plugin(&mut self, name: &str) -> anyhow::Result<()> {
        self.registry.validate_plugin_exists(name)?;
        
        // Call on_disable hook if exists
        self.call_plugin_hook(name, "on_disable")?;
        
        // Mark as disabled
        self.registry.disable(name);
        
        Ok(())
    }
    
    /// Load plugin Lua code
    fn load_plugin(&self, name: &str) -> anyhow::Result<()> {
        let plugin_info = self.registry.get_plugin(name)?;
        let entry_point = plugin_info
            .metadata
            .entry_point
            .as_ref()
            .unwrap_or(&"init.lua".to_string());
        
        let plugin_file = plugin_info.path.join(entry_point);
        
        // Load plugin code via Lua's require system
        // require("plugins.<name>")
        let _ = self.runtime.load_plugin(name, &plugin_file)?;
        
        // Call on_load hook
        self.call_plugin_hook(name, "on_load")?;
        
        Ok(())
    }
    
    /// Call plugin lifecycle hook
    fn call_plugin_hook(&self, name: &str, hook: &str) -> anyhow::Result<()> {
        // Call plugin.<hook>() if it exists
        // E.g., plugin.on_load() or plugin.on_disable()
        Ok(())
    }
    
    /// List all plugins with status
    pub fn list_plugins(&self) -> Vec<PluginStatus> {
        self.registry
            .plugins
            .iter()
            .map(|(name, info)| PluginStatus {
                name: name.clone(),
                version: info.metadata.version.clone(),
                enabled: self.registry.enabled.contains(name),
                loaded: info.loaded,
                dependencies: info.metadata.dependencies.len(),
            })
            .collect()
    }
    
    /// Get plugin status
    pub fn get_plugin_status(&self, name: &str) -> anyhow::Result<PluginStatus> {
        let info = self.registry.get_plugin(name)?;
        Ok(PluginStatus {
            name: name.to_string(),
            version: info.metadata.version.clone(),
            enabled: self.registry.enabled.contains(name),
            loaded: info.loaded,
            dependencies: info.metadata.dependencies.len(),
        })
    }
}

#[derive(Debug, Serialize)]
pub struct PluginStatus {
    pub name: String,
    pub version: String,
    pub enabled: bool,
    pub loaded: bool,
    pub dependencies: usize,
}
```

#### Plugin Registry

```rust
impl PluginRegistry {
    fn new() -> Self {
        PluginRegistry {
            plugins: HashMap::new(),
            enabled: HashSet::new(),
            disabled: HashSet::new(),
        }
    }
    
    fn add_plugin(&mut self, plugin_info: PluginInfo) {
        self.plugins.insert(plugin_info.metadata.name.clone(), plugin_info);
    }
    
    fn get_plugin(&self, name: &str) -> anyhow::Result<&PluginInfo> {
        self.plugins
            .get(name)
            .ok_or_else(|| anyhow!("Plugin not found: {}", name))
    }
    
    fn validate_plugin_exists(&self, name: &str) -> anyhow::Result<()> {
        self.get_plugin(name)?;
        Ok(())
    }
    
    fn check_compatibility(&self, name: &str) -> anyhow::Result<()> {
        let plugin = self.get_plugin(name)?;
        let compat = &plugin.metadata.compatibility;
        
        let niri_version = env!("CARGO_PKG_VERSION");
        
        // Check Niri version compatibility
        if compat.min_niri_version > niri_version {
            return Err(anyhow!(
                "Plugin requires Niri {} or later (you have {})",
                compat.min_niri_version,
                niri_version
            ));
        }
        
        if let Some(ref max) = compat.max_niri_version {
            if niri_version > max {
                return Err(anyhow!(
                    "Plugin is not compatible with Niri {} (max: {})",
                    niri_version,
                    max
                ));
            }
        }
        
        Ok(())
    }
    
    fn enable(&mut self, name: &str) {
        self.enabled.insert(name.to_string());
        self.disabled.remove(name);
    }
    
    fn disable(&mut self, name: &str) {
        self.disabled.insert(name.to_string());
        self.enabled.remove(name);
    }
}
```

---

### 2. Plugin Sandbox (`src/lua_extensions/plugin_sandbox.rs`)

#### Purpose
Isolate plugin code and manage permissions.

#### Implementation

```rust
pub struct PluginSandbox {
    plugin_name: String,
    permissions: PluginPermissions,
    lua_env: LuaTable,
}

impl PluginSandbox {
    /// Create sandbox for plugin
    pub fn new(
        lua: &Lua,
        plugin_name: &str,
        permissions: PluginPermissions,
    ) -> LuaResult<Self> {
        // Create isolated environment for plugin
        let env = lua.create_table()?;
        
        // Set up restricted niri.* API based on permissions
        if permissions.access_state {
            // Provide read-only state access
        }
        if permissions.access_input {
            // Provide input-related functions
        }
        if permissions.spawn_processes {
            // Provide process spawning
        }
        
        // Provide standard Lua libraries with restrictions
        let sandbox = PluginSandbox {
            plugin_name: plugin_name.to_string(),
            permissions,
            lua_env: env,
        };
        
        Ok(sandbox)
    }
    
    /// Load plugin code in sandbox
    pub fn load_code(&self, code: &str) -> LuaResult<()> {
        // Load plugin code with _ENV set to sandbox environment
        let chunk = self.lua_env.lua().load(code)
            .set_environment(self.lua_env.clone())?;
        
        chunk.exec()?;
        Ok(())
    }
    
    /// Call plugin function in sandbox
    pub fn call_function(&self, func_name: &str, args: &[LuaValue]) -> LuaResult<LuaValue> {
        if let Ok(func) = self.lua_env.get::<_, LuaFunction>(func_name) {
            func.call(args.to_vec())
        } else {
            Err(anyhow!("Function not found: {}", func_name).into())
        }
    }
}
```

---

### 3. Plugin API (`src/lua_extensions/plugin_api.rs`)

#### Purpose
Provide plugin-specific APIs for lifecycle and state management.

#### Plugin Lifecycle Hooks

```lua
-- ~/.config/niri/plugins/my-plugin/init.lua

local plugin = {}

-- Called when plugin is loaded
function plugin.on_load(plugin_api)
    plugin_api:log("Plugin loaded")
    
    -- Initialize plugin state
    local state = plugin_api:get_state()
    state.initialized = true
    plugin_api:set_state(state)
end

-- Called when plugin is enabled
function plugin.on_enable()
    niri.log("Plugin enabled")
end

-- Called when plugin is disabled
function plugin.on_disable()
    niri.log("Plugin disabled")
end

-- Called on Lua config reload
function plugin.on_reload()
    niri.log("Plugin reloaded")
end

-- Called before plugin is unloaded
function plugin.on_unload()
    niri.log("Plugin unloaded")
end

return plugin
```

#### Plugin State Management

```rust
pub struct PluginApi;

impl PluginApi {
    /// Register plugin-specific APIs to Lua
    pub fn register_to_lua(
        lua: &Lua,
        plugin_name: &str,
        state_dir: &Path,
    ) -> LuaResult<()> {
        let plugin_table = lua.create_table()?;
        
        // State management functions
        let get_state = {
            let state_dir = state_dir.to_path_buf();
            let plugin_name = plugin_name.to_string();
            lua.create_function(move |lua, ()| {
                let state_file = state_dir.join("state.json");
                if state_file.exists() {
                    let content = std::fs::read_to_string(&state_file)?;
                    let value: serde_json::Value = serde_json::from_str(&content)?;
                    json_to_lua(lua, &value)
                } else {
                    Ok(lua.create_table()?)
                }
            })?
        };
        plugin_table.set("get_state", get_state)?;
        
        let set_state = {
            let state_dir = state_dir.to_path_buf();
            lua.create_function(move |_, state: LuaTable| {
                let state_file = state_dir.join("state.json");
                let value = lua_to_json(&state)?;
                let content = serde_json::to_string_pretty(&value)?;
                std::fs::write(state_file, content)?;
                Ok(())
            })?
        };
        plugin_table.set("set_state", set_state)?;
        
        // Logging with plugin prefix
        let log = {
            let plugin_name = plugin_name.to_string();
            lua.create_function(move |_, msg: String| {
                println!("[{}] {}", plugin_name, msg);
                Ok(())
            })?
        };
        plugin_table.set("log", log)?;
        
        // Register hotkeys specific to plugin
        let register_hotkey = lua.create_function(|_, (key, action): (String, LuaFunction)| {
            // Register hotkey for this plugin
            Ok(())
        })?;
        plugin_table.set("register_hotkey", register_hotkey)?;
        
        let niri_table = lua.globals().get::<_, LuaTable>("niri")?;
        niri_table.set("plugin", plugin_table)?;
        
        Ok(())
    }
}

// Helper functions for Lua ↔ JSON conversion
fn lua_to_json(value: &LuaValue) -> anyhow::Result<serde_json::Value> {
    match value {
        LuaValue::Nil => Ok(serde_json::Value::Null),
        LuaValue::Boolean(b) => Ok(serde_json::Value::Bool(*b)),
        LuaValue::Integer(i) => Ok(serde_json::json!(*i)),
        LuaValue::Number(n) => Ok(serde_json::json!(*n)),
        LuaValue::String(s) => Ok(serde_json::Value::String(
            s.to_str()?.to_string(),
        )),
        LuaValue::Table(t) => {
            let mut obj = serde_json::Map::new();
            for pair in t.pairs::<String, LuaValue>() {
                let (k, v) = pair?;
                obj.insert(k, lua_to_json(&v)?);
            }
            Ok(serde_json::Value::Object(obj))
        }
        _ => Err(anyhow!("Cannot convert Lua value to JSON")),
    }
}

fn json_to_lua(lua: &Lua, value: &serde_json::Value) -> LuaResult<LuaValue> {
    match value {
        serde_json::Value::Null => Ok(LuaValue::Nil),
        serde_json::Value::Bool(b) => Ok(LuaValue::Boolean(*b)),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(LuaValue::Integer(i))
            } else if let Some(f) = n.as_f64() {
                Ok(LuaValue::Number(f))
            } else {
                Err(anyhow!("Invalid number").into())
            }
        }
        serde_json::Value::String(s) => Ok(LuaValue::String(lua.create_string(s)?)),
        serde_json::Value::Array(arr) => {
            let table = lua.create_table()?;
            for (i, v) in arr.iter().enumerate() {
                table.set(i + 1, json_to_lua(lua, v)?)?;
            }
            Ok(LuaValue::Table(table))
        }
        serde_json::Value::Object(obj) => {
            let table = lua.create_table()?;
            for (k, v) in obj.iter() {
                table.set(k.clone(), json_to_lua(lua, v)?)?;
            }
            Ok(LuaValue::Table(table))
        }
    }
}
```

---

### 4. Plugin Registry (`src/lua_extensions/plugin_registry.rs`)

#### Purpose
Central registry of available plugins for discovery and distribution.

#### Registry Format

```toml
# ~/.config/niri/plugins/registry.toml
# Auto-maintained by Niri plugin manager

[plugins."awesome-status-bar"]
version = "0.2.1"
author = "Jane Doe"
description = "A beautiful status bar for Niri"
repository = "https://github.com/user/awesome-status-bar"
license = "MIT"
min_niri_version = "25.0.0"

[plugins."awesome-status-bar".dependencies]
"awesome-lib" = ">=0.1.0"

[plugins."custom-layout"]
version = "0.1.0"
author = "John Doe"
description = "Custom tiling layout"
```

#### Registry Functions

```rust
pub struct PluginRegistry {
    plugins: HashMap<String, PluginEntry>,
    registry_file: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginEntry {
    pub version: String,
    pub author: Option<String>,
    pub description: Option<String>,
    pub repository: Option<String>,
    pub license: Option<String>,
    pub min_niri_version: String,
    pub dependencies: HashMap<String, String>,
}

impl PluginRegistry {
    /// Load registry from file
    pub fn load(path: PathBuf) -> anyhow::Result<Self> {
        let content = if path.exists() {
            std::fs::read_to_string(&path)?
        } else {
            "".to_string()
        };
        
        let plugins: HashMap<String, PluginEntry> = 
            toml::from_str(&content).unwrap_or_default();
        
        Ok(PluginRegistry {
            plugins,
            registry_file: path,
        })
    }
    
    /// Save registry to file
    pub fn save(&self) -> anyhow::Result<()> {
        let content = toml::to_string_pretty(&self.plugins)?;
        std::fs::write(&self.registry_file, content)?;
        Ok(())
    }
    
    /// Add plugin to registry
    pub fn add_plugin(&mut self, name: String, entry: PluginEntry) -> anyhow::Result<()> {
        self.plugins.insert(name, entry);
        self.save()?;
        Ok(())
    }
    
    /// Get plugin info
    pub fn get_plugin(&self, name: &str) -> Option<&PluginEntry> {
        self.plugins.get(name)
    }
    
    /// List all plugins in registry
    pub fn list_all(&self) -> Vec<(&String, &PluginEntry)> {
        self.plugins.iter().collect()
    }
    
    /// Resolve dependencies for plugin
    pub fn resolve_dependencies(
        &self,
        name: &str,
    ) -> anyhow::Result<Vec<String>> {
        let mut resolved = Vec::new();
        let mut visited = HashSet::new();
        
        self.resolve_deps_recursive(name, &mut resolved, &mut visited)?;
        
        Ok(resolved)
    }
    
    fn resolve_deps_recursive(
        &self,
        name: &str,
        resolved: &mut Vec<String>,
        visited: &mut HashSet<String>,
    ) -> anyhow::Result<()> {
        if visited.contains(name) {
            return Ok(());  // Already processed
        }
        
        visited.insert(name.to_string());
        
        if let Some(plugin) = self.get_plugin(name) {
            for dep_name in plugin.dependencies.keys() {
                self.resolve_deps_recursive(dep_name, resolved, visited)?;
            }
        }
        
        resolved.push(name.to_string());
        Ok(())
    }
}
```

---

## IPC Commands for Plugin Management

```
# List all plugins
$ niri msg plugin list
[
  {
    "name": "awesome-status-bar",
    "version": "0.2.1",
    "enabled": true,
    "loaded": true,
    "dependencies": 1
  },
  ...
]

# Enable plugin
$ niri msg plugin enable awesome-status-bar
{ "status": "success", "message": "Plugin enabled" }

# Disable plugin
$ niri msg plugin disable awesome-status-bar
{ "status": "success", "message": "Plugin disabled" }

# Get plugin info
$ niri msg plugin info awesome-status-bar
{
  "name": "awesome-status-bar",
  "version": "0.2.1",
  "author": "Jane Doe",
  "description": "A beautiful status bar for Niri",
  "repository": "https://github.com/user/awesome-status-bar",
  "license": "MIT",
  "enabled": true,
  "loaded": true,
  "dependencies": ["awesome-lib"]
}
```

---

## File Structure Summary

**New Files:**
- `src/lua_extensions/plugin_manager.rs` (250 lines)
- `src/lua_extensions/plugin_sandbox.rs` (100 lines)
- `src/lua_extensions/plugin_api.rs` (150 lines)
- `src/lua_extensions/plugin_registry.rs` (150 lines)

**Modified Files:**
- `src/lua_extensions/mod.rs` (+20 lines)
- `src/lua_extensions/runtime.rs` (+40 lines)
- `src/ipc/server.rs` (+50 lines - plugin IPC commands)

**Documentation:**
- `docs/LUA_TIER5_SPEC.md` (this file)
- `docs/LUA_PLUGIN_DEVELOPMENT.md` (plugin dev guide)

---

## Example Plugin

```lua
-- ~/.config/niri/plugins/window-counter/init.lua
local plugin = {}

function plugin.on_load(plugin_api)
    plugin_api:log("Window counter plugin loading")
    
    local state = plugin_api:get_state()
    if not state.window_count then
        state.window_count = 0
    end
    plugin_api:set_state(state)
end

function plugin.on_enable()
    niri.log("Window counter enabled")
    
    niri.on("window:open", function(event)
        local state = niri.plugin:get_state()
        state.window_count = state.window_count + 1
        niri.plugin:set_state(state)
        niri.plugin:log("Windows open: " .. state.window_count)
    end)
    
    niri.on("window:close", function(event)
        local state = niri.plugin:get_state()
        state.window_count = math.max(0, state.window_count - 1)
        niri.plugin:set_state(state)
        niri.plugin:log("Windows open: " .. state.window_count)
    end)
end

function plugin.on_disable()
    niri.log("Window counter disabled")
end

return plugin
```

---

## Testing Strategy

### Unit Tests

```rust
#[test]
fn test_plugin_discovery() {
    // Verify plugins are discovered correctly
}

#[test]
fn test_dependency_resolution() {
    // Test dependency resolution with circular deps
}

#[test]
fn test_version_compatibility() {
    // Verify version checking works
}

#[test]
fn test_state_persistence() {
    // Verify plugin state survives reload
}
```

---

## Success Criteria

✅ All plugins discoverable and loadable  
✅ Dependencies resolved correctly  
✅ Plugin state persists across reloads  
✅ IPC commands working  
✅ Version compatibility checked  
✅ All tests passing  

---

## References

- [Plugin Architecture Patterns](https://en.wikipedia.org/wiki/Plug-in_(computing))
- [Semantic Versioning](https://semver.org/)
- [JSON Schema Validation](https://json-schema.org/)
