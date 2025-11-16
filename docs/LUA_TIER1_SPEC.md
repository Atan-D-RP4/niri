# Tier 1 Specification: Module System & Plugin Foundation

**Duration:** Weeks 1-2  
**Estimated LOC:** 470 Rust + 100 documentation  
**Complexity:** Medium-High

## Overview

Tier 1 establishes the foundation for a modular, pluggable Lua ecosystem. It enables users to:
- Load Lua modules using standard `require()` function
- Organize code into multiple files
- Discover and load plugins from a standard directory
- React to Niri events with simple callbacks
- Automatically reload configuration when files change

This tier is critical because all subsequent tiers build upon these foundations.

---

## Architecture

```
Lua Module System:
  - require() function → ModuleLoader
  - Search paths: ~/.config/niri/lua/?.lua, ~/.config/niri/lua/?/init.lua
  - Module cache to prevent re-execution
  - Circular dependency detection

Plugin System:
  - Plugin directory: ~/.config/niri/plugins/
  - Each plugin is a directory: plugins/<name>/
  - Entry point: init.lua
  - Metadata: init.toml or init.json
  - Plugin environment table for isolation

Event System:
  - Basic event emitter infrastructure
  - Callback registration system
  - Currently: placeholder events (fully functional in Tier 4)
  - Foundation for Tier 4 integration

Hot Reload:
  - File watcher (inotify on Linux, kqueue on macOS, etc.)
  - IPC command: niri msg lua-hot-reload
  - Graceful state cleanup
  - Error recovery
```

---

## Detailed Specifications

### 1. Module Loader (`src/lua_extensions/module_loader.rs`)

#### Purpose
Implement Lua's `require()` function with Niri-specific search paths.

#### Key Functions

```rust
pub struct ModuleLoader {
    cache: HashMap<String, LuaValue>,
    search_paths: Vec<PathBuf>,
}

impl ModuleLoader {
    /// Create new module loader with default search paths
    pub fn new() -> anyhow::Result<Self>
    
    /// Add custom search path (e.g., for plugins)
    pub fn add_search_path(&mut self, path: impl Into<PathBuf>)
    
    /// Register require() function to Lua environment
    pub fn register_to_lua(lua: &Lua) -> LuaResult<()>
    
    /// Load and cache module
    fn load_module(&mut self, name: &str) -> LuaResult<LuaValue>
    
    /// Detect circular dependencies
    fn detect_circular_dependency(&self, name: &str) -> LuaResult<()>
}
```

#### Default Search Paths (in order)
1. `~/.config/niri/lua/?.lua` - module files
2. `~/.config/niri/lua/?/init.lua` - module packages
3. `~/.config/niri/plugins/*/lua/?.lua` - plugin libraries
4. Built-in modules (in Lua, provided by niri)

#### Module Naming Convention
- `require("module")` → searches for `module.lua` or `module/init.lua`
- `require("module.submodule")` → searches for `module/submodule.lua`
- Underscore equivalent: `require("my_module")` = `require("my-module")`

#### Error Handling

```
Example error messages:
- "Module 'foo' not found in search paths"
- "Circular require detected: foo → bar → foo"
- "Module 'foo' failed to load: [string "module.lua"]:5: attempt to index nil"
```

#### Caching Strategy
- Cache by module name (string)
- Clear cache on hot reload
- Prevent double-loading of same module
- Store either loaded Lua table or value

#### Testing

```
Tests to implement:
- test_module_loading_basic()
- test_module_with_dependencies()
- test_module_circular_dependency_detection()
- test_module_caching()
- test_search_path_resolution()
- test_module_not_found_error()
- test_module_with_init_lua()
```

#### Example Usage (in Lua)

```lua
-- config.lua
local utils = require("utils")
local config_helpers = require("config.helpers")

utils.log("Loading config")
config_helpers.apply_theme("dark")
```

```lua
-- lua/utils.lua
local utils = {}

function utils.log(msg)
    niri.log("[utils] " .. msg)
end

return utils
```

#### Implementation Notes
- Use `Lua::load()` for module code execution
- Create new Lua environment for each module (partial sandboxing)
- Store cache in `LuaRuntime` struct
- Update module loader initialization in `config.rs`

---

### 2. Plugin System (`src/lua_extensions/plugin_system.rs`)

#### Purpose
Discover, load, and manage Niri plugins from `~/.config/niri/plugins/` directory.

#### Plugin Directory Structure

```
~/.config/niri/plugins/
├── example-status-bar/
│   ├── init.lua              # Entry point
│   ├── init.toml             # Metadata (optional, auto-detected)
│   ├── lua/                  # Plugin-local libraries
│   │   └── utils.lua
│   └── assets/               # Plugin resources
│       └── icons/
│
├── example-window-manager/
│   ├── init.lua
│   └── init.json             # Alternative metadata format
│
└── my-custom-plugin/
    └── init.lua              # Metadata auto-generated from Lua
```

#### Metadata Format (init.toml)

```toml
[plugin]
name = "status-bar"
version = "0.1.0"
author = "John Doe"
description = "A custom status bar for Niri"
license = "MIT"

[plugin.dependencies]
"utils" = ">=0.1.0"  # Can depend on other plugins

[plugin.permissions]
# Tier 5 feature - which APIs the plugin can access
access_state = true
access_input = false
```

Or metadata auto-generated if not provided (Tier 5):
```toml
name = "my-custom-plugin"
version = "0.1.0"  # Default
description = ""   # Default empty
```

#### Key Structures

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginMetadata {
    pub name: String,
    pub version: String,
    pub author: Option<String>,
    pub description: Option<String>,
    pub license: Option<String>,
    pub dependencies: HashMap<String, String>,
    pub permissions: PluginPermissions,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PluginPermissions {
    pub access_state: bool,
    pub access_input: bool,
    // More permissions in Tier 5
}

pub struct PluginInfo {
    pub metadata: PluginMetadata,
    pub path: PathBuf,
    pub init_lua_path: PathBuf,
    pub lua_search_path: PathBuf,
}

pub struct PluginSystem {
    plugins: HashMap<String, PluginInfo>,
    search_dir: PathBuf,
    module_loader: Arc<Mutex<ModuleLoader>>,
}
```

#### Key Functions

```rust
impl PluginSystem {
    /// Create new plugin system
    pub fn new(search_dir: PathBuf, module_loader: Arc<Mutex<ModuleLoader>>) -> Self
    
    /// Discover all plugins in search directory
    pub fn discover_plugins() -> anyhow::Result<Vec<PluginInfo>>
    
    /// Load plugin metadata from init.toml or auto-detect from init.lua
    pub fn load_metadata(plugin_dir: &Path) -> anyhow::Result<PluginMetadata>
    
    /// Get plugin by name
    pub fn get_plugin(&self, name: &str) -> Option<&PluginInfo>
    
    /// List all discovered plugins
    pub fn list_plugins(&self) -> Vec<&PluginInfo>
    
    /// Initialize plugin search paths for module loader
    pub fn initialize_search_paths(&mut self, module_loader: &mut ModuleLoader)
}
```

#### Plugin Loading Process

1. **Discovery Phase:**
   - Scan `~/.config/niri/plugins/` for subdirectories
   - For each directory, check for `init.lua`
   - Load metadata from `init.toml`/`init.json` or auto-detect
   - Store plugin info (loaded later, not now)

2. **Initialization Phase (Tier 5):**
   - Actually load plugins via `require("plugins.<name>")`
   - Call `on_load()` hook if exists
   - Store plugin state

3. **Registration Phase (Tier 5):**
   - Register plugin event handlers
   - Register plugin hotkeys
   - Initialize plugin-specific APIs

#### Error Handling

```
- Ignore directories without init.lua (not an error)
- Warn on invalid metadata but continue with defaults
- Log plugin discovery results
- Provide clear error messages for:
  - Missing required fields in metadata
  - Invalid version format
  - Duplicate plugin names
```

#### Example Plugin (Tier 1)

```lua
-- ~/.config/niri/plugins/example-status-bar/init.lua
local plugin = {}

-- Plugin metadata (auto-detected if no init.toml)
plugin.name = "example-status-bar"
plugin.version = "0.1.0"

function plugin.init()
    niri.log("Status bar plugin initialized")
end

-- Called on hot reload (Tier 1)
function plugin.on_reload()
    niri.log("Status bar reloaded")
end

return plugin
```

#### Testing

```
Tests to implement:
- test_plugin_discovery()
- test_plugin_metadata_loading_toml()
- test_plugin_metadata_loading_json()
- test_plugin_metadata_auto_detect()
- test_invalid_plugin_directory()
- test_duplicate_plugin_names()
- test_plugin_search_path_setup()
```

---

### 3. Event Emitter Foundation (`src/lua_extensions/event_emitter.rs`)

#### Purpose
Provide infrastructure for Tier 4's full event system. In Tier 1, this is a placeholder that allows basic event registration but doesn't actually fire events yet. Tier 4 will integrate this with Niri core.

#### Key Structures

```rust
pub type EventCallback = LuaFunction;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EventType {
    // Window events (Tier 4)
    WindowOpen,
    WindowClose,
    // ... more events in Tier 4
    
    // Custom events
    Custom(u32),  // For user-defined events
}

pub struct EventEmitter {
    handlers: HashMap<EventType, Vec<EventCallback>>,
}

impl EventEmitter {
    pub fn new() -> Self
    
    /// Register event handler (Lua API: niri.on(event, callback))
    pub fn on(&mut self, event: EventType, handler: EventCallback) -> LuaResult<()>
    
    /// Register one-time handler (Lua API: niri.once(event, callback))
    pub fn once(&mut self, event: EventType, handler: EventCallback) -> LuaResult<()>
    
    /// Unregister handler (Lua API: niri.off(event, callback))
    pub fn off(&mut self, event: EventType, handler: EventCallback) -> LuaResult<()>
    
    /// Emit event (Tier 4 integration)
    pub fn emit(&self, event: EventType, data: &[LuaValue]) -> LuaResult<()>
    
    /// Register to Lua environment (provide niri.on/once/off/emit functions)
    pub fn register_to_lua(lua: &Lua) -> LuaResult<()>
}
```

#### Lua API (Tier 1 Placeholder)

```lua
-- Register event handler (currently not fired, but infrastructure ready)
niri.on("window:open", function(window_info)
    -- This will be called when a window opens (Tier 4)
end)

-- One-time handler
niri.once("window:close", function(window_info)
    -- Called once, then automatically unregistered
end)

-- Unregister handler
local handler = function() end
niri.on("workspace:activate", handler)
niri.off("workspace:activate", handler)

-- Custom events (Tier 5+)
niri.emit("custom:my_event", { custom_data = "value" })
```

#### Implementation Notes

- Store handlers by event type
- Use LuaFunction for type safety
- Support multiple handlers per event
- Handlers stored in Lua registry to prevent GC
- In Tier 4, emit() will be called from Niri core event handlers

#### Testing

```
Tests to implement (mostly placeholders):
- test_handler_registration()
- test_handler_unregistration()
- test_multiple_handlers_same_event()
- test_once_handler()
- test_custom_events()
```

---

### 4. Hot Reload (`src/lua_extensions/hot_reload.rs`)

#### Purpose
Watch Lua configuration files for changes and reload them without restarting Niri.

#### Key Structures

```rust
pub struct HotReloader {
    watch_paths: Vec<PathBuf>,
    watcher: Box<dyn Notify>,  // Or use notify crate
}

impl HotReloader {
    /// Create new hot reloader
    pub fn new(config_dir: PathBuf) -> anyhow::Result<Self>
    
    /// Start watching for file changes
    pub fn start_watching(&mut self) -> anyhow::Result<()>
    
    /// Reload Lua configuration
    pub fn reload_config(&self, runtime: &LuaRuntime) -> anyhow::Result<()>
    
    /// Handle file change event
    fn on_file_changed(&self, path: PathBuf)
}
```

#### Watched Files

```
Priority order:
1. ~/.config/niri/niri.lua (main config)
2. ~/.config/niri/init.lua (alternative name)
3. ~/.config/niri/lua/*.lua (all user modules)
4. ~/.config/niri/plugins/*/init.lua (plugin entry points)
5. ~/.config/niri/plugins/*/lua/*.lua (plugin modules)
```

#### Reload Process

1. **File Changed:** Inotify detects change
2. **Trigger Reload:** Log message "Lua config changed, reloading..."
3. **Save State:** Store current state for rollback
4. **Clear Cache:** Empty module cache
5. **Reload Config:** Execute niri.lua again
6. **Apply Changes:** Call `plugin.on_reload()` for each plugin
7. **On Error:** Rollback to previous state, log error, keep running

#### IPC Integration

```
Command: niri msg lua-hot-reload
Response:
{
    "status": "success|error",
    "message": "Config reloaded successfully" | "Error: ..."
}
```

#### Error Recovery

```rust
impl HotReloader {
    fn reload_with_rollback(&self, runtime: &LuaRuntime) -> anyhow::Result<()> {
        // Capture current state
        let old_state = capture_state(&runtime)?;
        
        // Try reload
        if let Err(e) = self.reload_config(runtime) {
            // Restore old state
            restore_state(&runtime, old_state)?;
            return Err(e);
        }
        
        Ok(())
    }
}
```

#### Dependency: `notify` crate

Add to Cargo.toml:
```toml
[workspace.dependencies]
notify = "6.1"
```

#### Testing

```
Tests to implement:
- test_file_change_detection()
- test_config_reload()
- test_error_recovery()
- test_plugin_reload_hooks()
- test_ipc_command()
```

---

## Integration with Existing Code

### Changes to `src/lua_extensions/mod.rs`

```rust
pub mod module_loader;
pub mod plugin_system;
pub mod event_emitter;
pub mod hot_reload;

pub use module_loader::ModuleLoader;
pub use plugin_system::{PluginSystem, PluginInfo, PluginMetadata};
pub use event_emitter::EventEmitter;
pub use hot_reload::HotReloader;
```

### Changes to `src/lua_extensions/runtime.rs`

```rust
pub struct LuaRuntime {
    lua: Lua,
    module_loader: Arc<Mutex<ModuleLoader>>,
    plugin_system: PluginSystem,
    event_emitter: EventEmitter,
    hot_reloader: Option<HotReloader>,
}

impl LuaRuntime {
    pub fn new() -> LuaResult<Self> {
        let lua = Lua::new();
        let module_loader = Arc::new(Mutex::new(ModuleLoader::new()?));
        let plugin_system = PluginSystem::new(
            dirs::config_dir().unwrap().join("niri/plugins"),
            module_loader.clone(),
        );
        let event_emitter = EventEmitter::new();
        
        // Register all to Lua
        ModuleLoader::register_to_lua(&lua)?;
        EventEmitter::register_to_lua(&lua)?;
        
        Ok(LuaRuntime {
            lua,
            module_loader,
            plugin_system,
            event_emitter,
            hot_reloader: None,
        })
    }
    
    pub fn initialize_hot_reload(&mut self) -> anyhow::Result<()> {
        let config_dir = dirs::config_dir().unwrap().join("niri");
        self.hot_reloader = Some(HotReloader::new(config_dir)?);
        Ok(())
    }
}
```

### Changes to `src/lua_extensions/config.rs`

```rust
impl LuaConfig {
    pub fn with_hot_reload(mut self, enabled: bool) -> Self {
        if enabled {
            // Initialize hot reloader
            let _ = self.runtime.initialize_hot_reload();
        }
        self
    }
}
```

---

## File Structure Summary

**New Files:**
- `src/lua_extensions/module_loader.rs` (150 lines)
- `src/lua_extensions/plugin_system.rs` (200 lines)
- `src/lua_extensions/event_emitter.rs` (120 lines)
- `src/lua_extensions/hot_reload.rs` (100 lines)

**Modified Files:**
- `src/lua_extensions/mod.rs` (+20 lines)
- `src/lua_extensions/runtime.rs` (+40 lines)
- `src/lua_extensions/config.rs` (+20 lines)
- `Cargo.toml` (+1 line: notify dependency)

**Documentation:**
- `docs/LUA_TIER1_SPEC.md` (this file)
- `docs/LUA_TIER1_IMPLEMENTATION_GUIDE.md` (how-to for developers)

---

## Testing Strategy

### Unit Tests (in each module)
- Module loading and caching
- Plugin discovery
- Event registration
- Hot reload state management

### Integration Tests
- Tier 1 features working together
- Module system + plugin system
- Hot reload with plugins

### Manual Testing Checklist
- [ ] Create multi-file Lua config and verify it loads
- [ ] Create plugin in `~/.config/niri/plugins/test/` and verify it loads
- [ ] Modify Lua file and verify hot reload triggers
- [ ] Verify `niri msg lua-hot-reload` works
- [ ] Test error recovery with invalid Lua syntax
- [ ] Test circular dependency detection

---

## Success Criteria

✅ Module loading works with `require()`  
✅ Plugins discovered from `~/.config/niri/plugins/`  
✅ File changes trigger hot reload  
✅ Event registration API available (Tier 4 fires events)  
✅ All tests passing  
✅ No performance regression (< 1ms hot reload)  
✅ Clear error messages for all failure modes  

---

## Open Questions for Implementation

1. Should we support plugin dependencies in Tier 1, or defer to Tier 5?
   - **Decision:** Defer to Tier 5, but store dependency info in Tier 1

2. Should hot reload be automatic or manual?
   - **Decision:** Automatic via file watcher, also available via IPC

3. Should we sandbox plugin code in Tier 1?
   - **Decision:** No, but infrastructure in place for Tier 5

4. What's the max size for plugin state that survives reload?
   - **Decision:** No limit in Tier 1, investigate limits in Tier 5

---

## Estimated Timeline

- **Research & Planning:** 1 day
- **Module Loader Implementation:** 2 days
- **Plugin System Implementation:** 2 days
- **Event Emitter & Hot Reload:** 2 days
- **Testing & Documentation:** 2 days
- **Buffer & Fixes:** 2 days

**Total:** ~2 weeks

---

## References

- [Lua 5.2 Module System](https://www.lua.org/manual/5.2/manual.html#pdf-require)
- [Neovim Plugin System](https://neovim.io/doc/user/lua.html#lua-plugin)
- [AwesomeWM Modules](https://awesomewm.org/doc/classes/)
- [mlua Documentation](https://docs.rs/mlua/latest/mlua/)
- [notify-rs File Watcher](https://docs.rs/notify/latest/notify/)
