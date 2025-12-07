## Build & Test Commands
- **Build**: `cargo build` (or `cargo build --release`)
- **Lint**: `cargo clippy --all-targets --all-features`
- **Format**: `cargo +nightly fmt --all` (check: `cargo +nightly fmt --all -- --check`)
- **Test all**: `cargo test`
- **Single test**: `cargo test test_name` or `cargo test module::test_name`
- **Update snapshots**: `cargo insta review` (uses insta for snapshot testing)

## Code Style
- **Imports**: Module-level granularity, grouped as std/external/crate (see rustfmt.toml)
- **Comments**: Wrap at 100 chars
- **Naming**: snake_case for functions/variables, CamelCase for types
- **Errors**: Use `anyhow` for error handling with `.context()` for context
- **Commits**: Small, focused, self-contained; each must build and pass tests
- **Clippy**: `new_without_default` is allowed; interior mutability ignored for Smithay types

---

## Niri Lua Crate Architecture

Comprehensive architecture of the Lua scripting system for Niri, organized in tiers. Foundation layer [1a-1f] provides core Lua infrastructure, Tier 2 [2a-2e] covers configuration API, and Tier 3 [3a-3e] provides runtime state access and IPC integration.

The niri-lua crate extends Niri with Lua scripting capabilities using mlua with the Luau dialect, providing a full-featured extension system inspired by the Astra project. It enables configuration, event handling, and runtime state access through Lua.

### 1. Foundation Layer - Core Lua Infrastructure

### 1a. Lua Runtime Initialization (`runtime.rs:31`)

Creates a new Lua runtime with safe standard library

```text
pub fn new() -> LuaResult<Self> {
    let lua = Lua::new();
    lua.load_std_libs(LuaStdLib::ALL_SAFE)?;
    Ok(Self {
        lua,
        event_system: None,
    })
}
```

**Purpose**: Initializes mlua with the Luau dialect and loads safe standard libraries (I/O, table, string, math, etc.). Excludes potentially unsafe features.

### 1b. Component Registration System (`runtime.rs:48`)

Generic system for registering Lua components

```text
pub fn register_component<F>(&self, action_callback: F) -> LuaResult<()>
where
    F: Fn(String, Vec<String>) -> LuaResult<()> + 'static,
{
    NiriApi::register_to_lua(&self.lua, action_callback)
}
```

**Purpose**: Provides a unified interface for registering components. The action callback allows Lua code to trigger compositor actions.

### 1c. LuaComponent Trait (`lib.rs:67`)

Core trait for extending Lua with custom functionality

```text
pub trait LuaComponent {
    fn register_to_lua<F>(lua: &Lua, action_callback: F) -> LuaResult<()>
    where
        F: Fn(String, Vec<String>) -> LuaResult<()> + 'static;
}
```

**Purpose**: Defines a standard interface for components to register themselves to the runtime. Implementers receive the lua context and action callback.

### 1d. Event System Creation (`event_system.rs:71`)

Creates the event system with shared handler storage

```text
pub struct EventSystem {
    handlers: SharedEventHandlers,
}

impl EventSystem {
    pub fn new(handlers: SharedEventHandlers) -> Self {
        Self { handlers }
    }
}
```

**Purpose**: Wraps the event handlers in a public interface for emitting events from the compositor core.

### 1e. Event API Registration (`event_system.rs:27`)

Registers `niri.on()`, `niri.once()`, and `niri.off()` functions

```text
niri_table.set(
    "on",
    lua.create_function(move |_, (event_type, callback): (String, LuaFunction)| {
        let mut h = handlers_on.lock();
        let handler_id = h.register_handler(&event_type, callback, false);
        Ok(handler_id)
    })?,
)?;
```

**Purpose**: Exposes event registration to Lua. Supports persistent handlers (`on`), one-time handlers (`once`), and removal (`off`).

### 1f. Script Loading (`runtime.rs:98`)

Loads and executes Lua scripts from files

```text
pub fn load_file<P: AsRef<Path>>(&self, path: P) -> LuaResult<LuaValue> {
    let code = std::fs::read_to_string(path)
        .map_err(|e| LuaError::external(format!("Failed to read Lua file: {}", e)))?;
    self.lua.load(code).eval()
}
```

**Purpose**: Reads a Lua script file and executes it in the runtime. Errors are returned as LuaError for proper error reporting.

### 2. Tier 2 - Configuration API

### 2a. Configuration API Registration (`config_api.rs:18`)

Registers the `niri.config` table with all configuration subsystems

```text
pub fn register_to_lua(lua: &Lua, config: &Config) -> LuaResult<()> {
    let niri_table: LuaTable = globals.get("niri")?;
    let config_table = lua.create_table()?;
    
    Self::register_animations(lua, &config_table, &config.animations)?;
    Self::register_input(lua, &config_table, &config.input)?;
    Self::register_layout(lua, &config_table, &config.layout)?;
    // ... other subsystems
    
    niri_table.set("config", config_table)?;
}
```

**Purpose**: Exposes the complete niri-config structure to Lua. All configuration values are read-only through this API.

### 2b. Animation Configuration Exposure (`config_api.rs:53`)

Provides Lua access to animation settings

```text
fn register_animations(
    lua: &Lua,
    config_table: &LuaTable,
    anim_config: &Animations,
) -> LuaResult<()> {
    let animations = lua.create_table()?;
    animations.set("off", anim_config.off)?;
    animations.set("slowdown", anim_config.slowdown)?;
    
    // Individual animation types
    let ws_switch = lua.create_table()?;
    Self::set_animation_values(lua, &ws_switch, &anim_config.workspace_switch.0)?;
    animations.set("workspace_switch", ws_switch)?;
    // ... more animation types
}
```

**Purpose**: Converts Rust animation configuration to Lua tables. Each animation type (workspace switch, window open, etc.) is exposed with its duration and curve.

### 2c. Input Configuration Exposure (`config_api.rs`)

Provides access to keyboard and input settings

```text
fn register_input(
    lua: &Lua,
    config_table: &LuaTable,
    input_config: &Input,
) -> LuaResult<()> {
    // Keyboard layouts
    // Scroll methods
    // Focus and warping settings
}
```

**Purpose**: Exposes input handling configuration including keyboard layouts, scroll behavior, and focus modes.

### 2d. Layout Configuration Exposure (`config_api.rs`)

Provides access to window layout settings

```text
fn register_layout(
    lua: &Lua,
    config_table: &LuaTable,
    layout_config: &Layout,
) -> LuaResult<()> {
    // Column width settings
    // Gaps and borders
    // Focus ring settings
}
```

**Purpose**: Exposes layout engine configuration for reading current tiling settings, gaps, borders, and focus indicators.

### 2e. Output Configuration Exposure (`config_api.rs`)

Provides access to monitor and display settings

```text
fn register_output(
    lua: &Lua,
    config_table: &LuaTable,
    outputs_config: &Outputs,
) -> LuaResult<()> {
    // Per-output settings
    // Position and scaling
    // Refresh rate and VRR
}
```

**Purpose**: Exposes monitor configurations including position, scaling, refresh rates, and variable refresh rate settings.

### 3. Tier 3 - Runtime State Access and IPC

### 3a. Runtime API Setup (`runtime_api.rs`)

Registers the `niri.state` table for querying live compositor state

```text
pub struct RuntimeApi<S> {
    // Reference to compositor state
}

pub fn register_runtime_api<S>(lua: &Lua, api: RuntimeApi<S>) -> LuaResult<()>
where
    S: CompositorState + 'static,
{
    // Register Lua functions for state queries
}
```

**Purpose**: Provides Lua with read-only access to dynamic compositor state (windows, workspaces, outputs, etc.).

### 3b. Event Data Structures (`event_data.rs`)

Defines Lua-compatible event data types

```text
pub struct WindowEventData {
    pub id: u64,
    pub title: String,
    pub app_id: String,
    pub workspace_id: u64,
}

pub struct WorkspaceEventData {
    pub id: u64,
    pub is_active: bool,
    pub is_focused: bool,
}
```

**Purpose**: Provides type-safe event data structures that can be converted to Lua tables when events are emitted.

### 3c. Event Emitter (`event_emitter.rs`)

Emits events from the compositor to Lua handlers

```text
pub struct EventEmitter {
    // Holds references to event system
}

impl EventEmitter {
    pub fn emit_window_event(&self, data: WindowEventData) -> LuaResult<()> {
        // Convert data to Lua table
        // Emit to event system
    }
}
```

**Purpose**: Converts compositor events (window open, workspace switch, etc.) to Lua tables and emits them through the event system.

### 3d. IPC Bridge (`ipc_bridge.rs`)

Bridges IPC requests to Lua execution

```text
pub struct IpcBridge {
    // Runtime reference
}

impl IpcBridge {
    pub fn execute_lua(&self, code: String) -> Result<String> {
        // Execute Lua code in runtime
        // Return result as JSON
    }
}
```

**Purpose**: Allows external IPC clients to execute Lua code via `Request::ExecuteLua`, with results returned over IPC.

### 3e. IPC REPL (`ipc_repl.rs`)

Interactive Lua REPL over IPC socket

```text
pub struct IpcLuaExecutor {
    // REPL state
}

impl IpcLuaExecutor {
    pub fn eval(&mut self, line: &str) -> Result<String> {
        // Evaluate line in REPL context
        // Return result or error
    }
}
```

**Purpose**: Provides an interactive REPL for Lua code execution through IPC, useful for debugging and scripting.

### 4. Configuration Conversion

### 4a. Lua to KDL Configuration (`config_converter.rs`)

Converts Lua configuration to niri-config structures

```text
pub fn apply_lua_config(lua: &Lua, config: &mut Config) -> LuaResult<()> {
    // Read config table from Lua
    // Apply values to niri-config Config struct
}
```

**Purpose**: Enables Lua scripts to define or modify configuration at startup before the compositor starts.

### 4b. Type Extractors (`extractors.rs`)

Safely extracts and converts Lua values to Rust types

```text
pub trait FromLua: Sized {
    fn from_lua(value: LuaValue, lua: &Lua) -> LuaResult<Self>;
}
```

**Purpose**: Provides type-safe extraction of configuration values from Lua tables, with proper error handling.

### 4c. Validators (`validators.rs`)

Validates extracted configuration values

```text
pub fn validate_color(value: &str) -> Result<()> {
    // Check valid color format
}

pub fn validate_animation_duration(value: u32) -> Result<()> {
    // Check reasonable bounds
}
```

**Purpose**: Ensures configuration values are within valid ranges before applying them to the compositor.

### 5. Plugin and Module System

### 5a. Plugin Manager (`plugin_system.rs`)

Manages loading and initialization of Lua plugins

```text
pub struct PluginManager {
    // Plugin registry
}

impl PluginManager {
    pub fn load_plugin(&mut self, path: &Path) -> LuaResult<()> {
        // Load plugin from file
        // Register with runtime
    }
}
```

**Purpose**: Allows loading of external Lua plugins that extend Niri's functionality.

### 5b. Module Loader (`module_loader.rs`)

Handles `require()` and module resolution for Lua

```text
pub struct ModuleLoader {
    search_paths: Vec<PathBuf>,
}

impl ModuleLoader {
    pub fn setup_module_paths(&self, lua: &Lua) -> LuaResult<()> {
        // Configure Lua package.path
    }
}
```

**Purpose**: Provides standard Lua module loading with Niri-specific search paths for plugins and libraries.

### 5c. Hot Reload (`hot_reload.rs`)

Detects and reloads Lua configuration on file changes

```text
pub struct HotReloader {
    file_watcher: RecommendedWatcher,
}

impl HotReloader {
    pub fn start_watching(&mut self, path: &Path) -> Result<()> {
        // Watch config file for changes
        // Reload on modification
    }
}
```

**Purpose**: Enables live reloading of Lua configuration without restarting the compositor.

### 6. Type System and Lua Types

### 6a. Lua Animation Types (`lua_types.rs`)

Wraps animation configuration for Lua

```text
pub struct LuaAnimation {
    pub duration_ms: u32,
    pub curve: String, // "ease-out", "spring", etc.
}
```

**Purpose**: Provides type-safe animation configuration that can be passed to Lua and back.

### 6b. Lua Window Rules (`lua_types.rs`)

Window matching and configuration in Lua

```text
pub struct LuaWindowRule {
    pub matches: Vec<String>,
    pub actions: Vec<String>,
}
```

**Purpose**: Allows Lua to define window rules for automatic window configuration.

### 6c. Lua Filters and Gestures (`lua_types.rs`)

Gesture and filter definitions in Lua

```text
pub struct LuaGesture {
    pub gesture_type: String,
    pub handler: LuaFunction,
}

pub struct LuaFilter {
    pub predicate: LuaFunction,
}
```

**Purpose**: Enables Lua-defined gestures and filters for extending input handling.

### 7. Testing Support

### 7a. Test Utilities (`test_utils.rs`)

Helpers for testing Lua scripts

```text
#[cfg(test)]
pub mod test_utils {
    pub fn create_test_runtime() -> LuaResult<LuaRuntime> {
        // Create runtime with test config
    }
}
```

**Purpose**: Provides utilities for writing tests of Lua functionality without a full compositor.

### 8. Architecture Layers Summary

- **Foundation (Tier 1)**: Runtime creation, component registration, event API
- **Configuration (Tier 2)**: Read-only access to all KDL-based configuration
- **Runtime (Tier 3)**: Live state queries, event data, IPC execution
- **Extensibility**: Plugin system, module loading, hot reloading

This tiered architecture allows different levels of Lua integration:
- **Basic**: Scripts that read configuration and handle events
- **Advanced**: Scripts that query runtime state and control the compositor
- **Expert**: External IPC clients executing arbitrary Lua code
