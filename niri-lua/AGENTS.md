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
    event_loop: LoopHandle<'static, S>,
}

pub fn register_runtime_api<S>(lua: &Lua, api: RuntimeApi<S>) -> LuaResult<()>
where
    S: CompositorState + 'static,
{
    // Register Lua functions for state queries
}
```

**Purpose**: Provides Lua with read-only access to dynamic compositor state (windows, workspaces, outputs, etc.).

#### Dual-Mode Query Architecture

The runtime API uses two different execution modes to avoid deadlocks:

**1. Event Handler Mode (synchronous)**
When called from within an event handler (e.g., `niri.events:on("window:open", ...)`):
- Uses pre-captured `StateSnapshot` stored in thread-local `EVENT_CONTEXT_STATE`
- Avoids deadlock that would occur if we used idle callbacks while the event loop waits for Lua
- Fast: No cross-thread communication needed

```text
thread_local! {
    static EVENT_CONTEXT_STATE: RefCell<Option<StateSnapshot>> = RefCell::new(None);
}
```

**2. Normal Mode (async via idle callback)**
When called from REPL, timers, or other non-event contexts:
- Creates a channel and sends query via `insert_idle()` to event loop
- Main thread handler runs with State access, sends result back
- Lua blocks waiting for response (from Lua's perspective)

```text
fn query<F, T>(&self, f: F) -> Result<T, String>
where F: FnOnce(&mut S, Sender<T>) + 'static
{
    let (tx, rx) = bounded(1);
    self.event_loop.insert_idle(move |state| { f(state, tx); });
    rx.recv_blocking()
}
```

#### Snapshot Staleness Limitation

**Important**: Event handlers see pre-captured snapshots, NOT live state after their own actions.

Example problem:
```lua
niri.events:on("window:open", function(data)
    niri.action:move_window_to_workspace({ id = 2 })
    -- BUG: niri.state.windows() still shows window on original workspace!
    local windows = niri.state.windows()
end)
```

**Mitigation strategies**:
1. Use event data directly (e.g., `data.window_id`) rather than re-querying
2. Schedule follow-up queries via timers: `niri.utils.defer(function() ... end)`
3. For multi-action scenarios, chain through separate event handlers

#### Planned Improvements (Runtime API)
- `get_window(id)` - Targeted window query by ID (avoids filtering full list)
- `get_workspace(ref)` - Query by ID, index, or name  
- `get_output(name)` - Output-specific query
- `focused_workspace()` - Direct access to active workspace
- `focused_output()` - Direct access to active output
- `niri.state.watch(path_or_selector, callback)` - Reactive state subscriptions (see LUA_IMPLEMENTATION_ROADMAP.md)

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

### 3c. Events Proxy (`events_proxy.rs`)

Bridges compositor events to Lua handlers

```text
pub fn register_events_proxy(lua: &Lua) -> LuaResult<()>
pub fn emit_event(lua: &Lua, event_type: &str, data: LuaValue) -> LuaResult<()>
```

**Purpose**: Registers the `niri.events` API in Lua and provides functionality to emit compositor events (window open/close, workspace switch, etc.) to registered Lua handlers.

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

### 5. Plugin and Module System (Future Work - Tier 5)

> **Note**: The plugin system and module loader are intentionally implemented but not yet integrated into the compositor. These are planned for Tier 5 and will enable external Lua plugins.

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
- **Async Primitives (Tier 4)**: Timers, scheduled callbacks, loop integration
- **Extensibility (Tier 5 - Future)**: Plugin system, module loading, hot reloading

This tiered architecture allows different levels of Lua integration:
- **Basic**: Scripts that read configuration and handle events
- **Advanced**: Scripts that query runtime state and control the compositor
- **Expert**: External IPC clients executing arbitrary Lua code

### 9. Current Implementation Status

#### Fully Working
- Event system: 23+ events wired with centralized emission in refresh cycle
- Config API: Read-only exposure of all configuration sections
- Reactive config proxy: Lua can modify config via `niri.config.*`
- Action proxy: All compositor actions accessible via `niri.action`
- Timer/loop API: `niri.loop.new_timer()`, `niri.loop.now()`, `niri.schedule()`
- State queries: `niri.state.windows()`, `focused_window()`, `workspaces()`, `outputs()`
- REPL: `niri msg lua -- 'code'` executes Lua with full state access
- API registry with LSP type generation (`types/api.lua`)

#### Compositor Integration (`src/lua_integration.rs`)
The Lua setup logic is consolidated in `src/lua_integration.rs`:
- `load_lua_config()` - Loads and applies Lua config (with dirty flag check)
- `create_action_channel()` - Creates calloop channel (with `advance_animations()`)
- `setup_runtime()` - Registers RuntimeApi, ConfigWrapper, ActionProxy
- `execute_pending_actions()` - Runs deferred actions from config load
- `is_lua_config_active()` - Checks if Lua runtime is present

This reduces ~150 lines of Lua code in `main.rs` to ~12 lines of function calls.

#### Event Emission Architecture (`src/lua_event_hooks.rs`)
Events are emitted from centralized locations in the refresh cycle:
- **Workspace events**: Detected in `ext_workspace.rs` refresh via `WorkspaceRefreshResult`
- **Overview events**: Detected in `niri.rs` refresh cycle via `LuaEventState`
- **Layout mode events**: Detected in `niri.rs` refresh cycle via `prev_floating_active`

This ensures events fire regardless of trigger source (keybindings, touch, IPC).

#### Intentionally Deferred (Tier 5)
- Plugin system (`plugin_system.rs`) - infrastructure ready, not integrated
- Module loader (`module_loader.rs`) - infrastructure ready, not integrated
