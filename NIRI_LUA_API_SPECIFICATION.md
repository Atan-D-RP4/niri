# Niri Lua API: Well-Engineered Architecture Specification

## Executive Summary

This specification defines a well-engineered Lua API architecture for the Niri Wayland compositor, combining the best patterns from successful Lua-embedded systems while addressing fundamental architectural constraints. The design prioritizes live state access, predictable behavior, and clean integration boundaries while maintaining Niri's performance and safety characteristics.

**Key Design Decision**: Like Neovim, this specification explicitly chooses NOT to implement hot reloading of Lua configuration. This decision prioritizes predictability, safety, and simpler implementation over convenience.

---

## 1. Core Design Philosophy

### 1.1 Guiding Principles

**Live State First**: All Lua operations work on live compositor state, eliminating snapshot-based stale data issues common in event handlers.

**Predictable Over Convenient**: Like Neovim, we choose explicit manual operations over automatic hot reload. This prevents subtle bugs from stale closures, partial state, and reference corruption.

**Type Safety at Boundaries**: Strong typing between Rust and Lua with compile-time guarantees while preserving Lua's dynamic ergonomics.

**Performance-Oriented**: Critical paths optimized for minimal indirection and maximum throughput.

**Security-Conscious**: Safe defaults with explicit opt-in for potentially dangerous operations.

### 1.2 Architectural Influences

- **Neovim**: Comprehensive API coverage, event system design, **no hot reload philosophy**
- **AwesomeWM**: Object-oriented patterns and signal-based communication
- **Pinnacle**: Rust-Lua bridge patterns, state management
- **WezTerm**: Configuration structure and validation patterns

---

## 2. System Architecture

### 2.1 Multi-Layer Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    Lua API Surface                      │
├─────────────────────────────────────────────────────────────┤
│  Configuration Layer  │  Runtime Layer  │  Action Layer │
├─────────────────────────────────────────────────────────────┤
│               Event System Integration                    │
├─────────────────────────────────────────────────────────────┤
│                  Rust Core Bridge                        │
├─────────────────────────────────────────────────────────────┤
│                 Niri Compositor Core                     │
└─────────────────────────────────────────────────────────────┘
```

### 2.2 Runtime Integration Model

**Single-Threaded Luau Runtime**: Main thread execution with interrupt-based timeout protection
**Channel-Based Communication**: Actions flow via `calloop::channel<Action>` for clean boundaries
**State Handle Pattern**: Direct read access to compositor state via `StateHandle`
**Event Bridge System**: Bidirectional event flow between compositor core and Lua handlers

### 2.3 Configuration Model

**Primary Configuration**: KDL configuration remains foundation for core settings
**Lua Overlay**: `init.lua` detected at runtime provides dynamic configuration and scripting
**Loading Strategy**: KDL loaded first, then `init.lua` executed to overlay/extend
**No Hot Reload**: Configuration changes require compositor restart (like Neovim)
**Migration Path**: Gradual migration from KDL to Lua with hybrid operation support

---

## 3. Lua API Surface Design

### 3.1 Core API Structure

```lua
-- Global niri object with domain-specific modules
local niri = {
    -- Configuration access (read/write)
    config = {
        layout = LayoutConfig,
        input = InputConfig,
        output = OutputConfig,
        binds = BindCollection,
        workspaces = WorkspaceCollection,
        -- Domain-specific configuration objects
    },
    
    -- Runtime state queries (live access)
    state = {
        windows = WindowCollection,
        workspaces = WorkspaceState,
        outputs = OutputState,
        focused_window = WindowHandle,
        cursor_position = {x, y},
        -- Live state access methods
    },
    
    -- Action execution
    action = {
        window = WindowActions,
        workspace = WorkspaceActions,
        layout = LayoutActions,
        system = SystemActions,
        spawn = ProcessSpawn,
        -- Action methods
    },
    
    -- Event system
    event = {
        on = EventSubscription,
        emit = EventEmission,
        signal = SignalSystem,
        -- Event management
    },
    
    -- Utilities and libraries
    util = {
        timer = TimerSystem,
        async = AsyncPrimitives,
        os = OSUtilities,
        debug = DebugTools,
    }
}
```

### 3.2 Configuration API Design

**Hybrid Access Pattern**:
```lua
-- Static KDL configuration access (read-only)
local gaps = niri.config.layout.gaps

-- Dynamic Lua configuration (read/write)
niri.config.layout.gaps = 8
niri.config.layout.center_focused_column = true

-- Collection management
niri.config.workspaces:add({name = "coding"})
niri.config.workspaces:remove("1")
local workspaces = niri.config.workspaces:list()

-- Reactive property changes
niri.config.layout:watch("gaps", function(new_gaps)
    print("Layout gaps changed to:", new_gaps)
end)
```

### 3.3 State API Design (Live Access)

**Window Management**:
```lua
-- Query windows with filters (live state)
local focused_window = niri.state.windows:focused()
local firefox_windows = niri.state.windows:filter({app_id = "firefox*"})
local workspace_windows = niri.state.windows:in_workspace("1")

-- Direct window manipulation
focused_window:move_to_workspace("2")
focused_window:set_floating(true, {x = 100, y = 100, width = 800, height = 600})

-- Window properties (live access)
print(focused_window.title, focused_window.app_id, focused_window.is_floating)
```

**Workspace Management**:
```lua
-- Workspace state (live access)
local current_workspace = niri.state.workspaces:current()
local workspace_list = niri.state.workspaces:list()

-- Workspace operations
niri.action.workspace.activate("2")
niri.action.workspace:next()
niri.action.workspace:previous()

-- Workspace properties
print(current_workspace.name, current_workspace.index, current_workspace.output)
```

### 3.4 Action System Design

**Window Actions**:
```lua
-- Direct action calls
niri.action.window.focus_right()
niri.action.window.move_up()
niri.action.window:toggle_floating()

-- Batch operations
niri.action.window.move_to_workspace_and_focus("2")
niri.action.window:set_size_and_position({width = 800, height = 600, x = 100, y = 100})
```

**Process Management**:
```lua
-- Simple process spawning
local result = niri.action.spawn("firefox", {cwd = "/home/user"})
if result.success then
    print("Process started with PID:", result.pid)
end

-- Advanced process management with callbacks
local process = niri.action.spawn("git", {
    args = {"status"},
    cwd = "/home/user/project",
    on_stdout = function(line) print("stdout:", line) end,
    on_stderr = function(line) print("stderr:", line) end,
    on_exit = function(code) print("exited:", code) end
})

-- Streaming process output
process:read_line(function(line)
    -- Process each line as it arrives
end)
```

### 3.5 Event System Design

**Signal-Based Architecture**:
```lua
-- Event subscription (signal-based)
niri.event.on("window_open", function(window_handle)
    print("Window opened:", window_handle.title)
end)

niri.event.on("workspace_activate", function(workspace_name, index)
    print("Workspace activated:", workspace_name, index)
end)

-- Pattern matching and filtering
niri.event.on("key_press", function(event)
    if event.modifiers.ctrl and event.key == "q" then
        -- Custom key handling
    end
end)

-- One-time event handlers
niri.event.once("niri_ready", function()
    print("Niri ready, initializing custom configuration")
end)
```

**Custom Event Emission**:
```lua
-- Emit custom events for user scripts
niri.event.emit("my_custom_event", {data = "example"})

-- Event emission in other event handlers (combining events)
niri.event.on("window_open", function(window)
    if window.app_id == "firefox" then
        niri.event.emit("browser_opened", {window = window})
    end
end)
```

---

## 4. Integration Architecture

### 4.1 Compositor Integration Points

**State Integration**:
```rust
// In Niri core structure
pub struct Niri<M: GeometryHandler> {
    pub lua_runtime: Option<LuaRuntime>,
    pub state_handle: StateHandle,
    // ... other fields
}

// StateHandle provides thread-safe read access
pub struct StateHandle {
    state: Arc<Mutex<State>>,
    // Methods for live state queries
}

impl StateHandle {
    pub fn with_state<F, R>(&self, f: F) -> R 
    where F: FnOnce(&State) -> R
    {
        let state = self.state.lock().unwrap();
        f(&state)
    }
}
```

**Event Integration**:
```rust
// Trait-based event emission from compositor core
pub trait NiriLuaEvents {
    fn emit_window_open(&self, window: &Window);
    fn emit_workspace_activate(&self, workspace: &Workspace);
    fn emit_key_press(&self, key_event: &KeyEvent);
}

// Implementation in main compositor
impl NiriLuaEvents for Niri {
    fn emit_window_open(&self, window: &Window) {
        if let Some(lua) = &self.lua_runtime {
            lua.emit_event("window_open", window.to_lua_data());
        }
    }
}
```

**Action Integration**:
```rust
// Action channel from Lua to compositor
pub struct LuaActionReceiver {
    receiver: calloop::channel::Receiver<Action>,
}

impl LuaActionReceiver {
    pub fn process_actions(&mut self, state: &mut State) {
        while let Ok(action) = self.receiver.try_recv() {
            state.do_action(action);
            state.advance_animations(); // Smooth feedback
        }
    }
}
```

### 4.2 Configuration Loading Architecture

**Hybrid Configuration Loading**:
```rust
pub struct ConfigLoader {
    kdl_config: Config,
    lua_runtime: Option<LuaRuntime>,
}

impl ConfigLoader {
    pub fn load(config_path: &Path) -> Result<Self, ConfigError> {
        // Always load KDL configuration as base
        let kdl_config = Config::from_kdl_file(config_path)?;
        
        // Check for init.lua in same directory
        let lua_path = config_path.with_file_name("init.lua");
        let lua_runtime = if lua_path.exists() {
            let runtime = LuaRuntime::new()?;
            runtime.execute_config(&lua_path)?;
            Some(runtime)
        } else {
            None
        };
        
        Ok(Self {
            kdl_config,
            lua_runtime,
        })
    }
}
```

### 4.3 No Hot Reload: Design Rationale

**Decision**: Like Neovim, Niri explicitly chooses NOT to implement automatic hot reloading of Lua configuration.

**Why AwesomeWM's Approach Doesn't Fit**:
```
AwesomeWM Hot Reload (awesome.restart()):
├── Uses execvp() to restart entire process
├── Preserves window order via X11 properties
├── Brief visual disruption (windows flash)
├── All Lua state completely lost
└── Simple but jarring user experience
```

**Why Neovim's Approach Is Better for Niri**:
```
Neovim No-Reload Philosophy:
├── Complex state (LSP, buffers, plugins) makes hot reload dangerous
├── Partial reloads cause subtle bugs with closures/upvalues
├── Predictable behavior more important than convenience
├── Users manually :source files when needed
└── Clean mental model: restart = fresh state
```

**Technical Challenges Hot Reload Would Introduce**:

| Challenge | Description | Risk Level |
|-----------|-------------|------------|
| **Closure Corruption** | Functions retain references to old values after reload | Critical |
| **Upvalue Staleness** | Captured variables become inconsistent | Critical |
| **Coroutine State** | Suspended coroutines reference stale functions | High |
| **Module Ordering** | Wrong dependency order causes undefined references | High |
| **Memory Leaks** | Multiple reload cycles accumulate unreferenced objects | Medium |
| **Timer/Callback Orphans** | Registered callbacks reference old function instances | Medium |
| **Partial State** | Half-applied config leaves compositor inconsistent | Critical |

**Concrete Example of Closure Corruption**:
```lua
-- init.lua (version 1)
local counter = 0
niri.event.on("window_open", function(win)
    counter = counter + 1  -- Captures 'counter' in closure
    print("Window count:", counter)
end)

-- After "hot reload" to version 2:
-- The event handler still references OLD counter variable
-- New code has a DIFFERENT counter variable
-- Result: unpredictable behavior, impossible to debug
```

**Niri's Approach**:
```rust
// No file watching, no automatic reload
// User must restart niri for config changes
// This matches Neovim's philosophy:
// - Predictable behavior
// - No hidden state corruption
// - Clean mental model
// - Simpler implementation

pub struct LuaRuntime {
    lua: Lua,
    // No watcher, no reload state
}

impl LuaRuntime {
    pub fn new() -> Result<Self, LuaError> {
        let lua = Lua::new();
        // One-time initialization
        Ok(Self { lua })
    }
    
    // No reload_config() method - intentionally absent
}
```

**Manual Reload Pattern (Like Neovim)**:
```lua
-- User can manually source files during development:
-- This is explicit and predictable

-- In Neovim: :source ~/.config/nvim/init.lua
-- In Niri:   niri msg action quit && niri

-- Or provide a convenience action:
niri.action.restart()  -- Cleanly restarts compositor
```

**Benefits of No Hot Reload**:

1. **Predictability**: Restart = completely fresh state, no hidden corruption
2. **Simplicity**: No complex state tracking, file watching, or rollback logic
3. **Debuggability**: Issues always reproducible from clean start
4. **Performance**: No overhead from change detection or reload infrastructure
5. **Safety**: No risk of leaving compositor in inconsistent state

**User Experience Mitigation**:
- Fast compositor startup (< 500ms target)
- Session restoration via Wayland protocols preserves window state
- Clear error messages on startup if config is invalid
- `niri msg action quit` followed by `niri` is the reload pattern

---

## 5. Memory and Performance Architecture

### 5.1 Memory Management Strategy

**Shared Ownership Model**:
```rust
// Lua objects hold Arc references to compositor state
pub struct WindowHandle {
    window_id: u64,
    state: Arc<Mutex<State>>,
}

impl WindowHandle {
    pub fn title(&self) -> String {
        let state = self.state.lock().unwrap();
        state.windows
            .get(&self.window_id)
            .map(|w| w.title.clone())
            .unwrap_or_default()
    }
}
```

**Garbage Collection Integration**:
```rust
// Automatic cleanup when Lua objects are GC'd
impl UserData for WindowHandle {
    fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("title", |_, this, ()| {
            Ok(this.title()) // Direct state access
        });
    }
}

// Weak references for event handlers to prevent cycles
pub struct EventHandler {
    callback: LuaFunction,
    // Weak reference to prevent memory leaks
    runtime_weak: Weak<Mutex<LuaRuntime>>,
}
```

### 5.2 Performance Optimizations

**Direct Field Access**:
```rust
// Configuration access without indirection
impl UserData for LayoutConfig {
    fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("gaps", |_, this, ()| {
            let config = this.config.borrow();
            Ok(config.layout.gaps) // Direct field access
        });
        
        methods.add_method_set("gaps", |_, this, value: f64| {
            let mut config = this.config.borrow_mut();
            config.layout.gaps = value;
            this.mark_dirty(DirtyFlag::Layout);
            Ok(())
        });
    }
}
```

**Efficient State Queries**:
```rust
// Optimized window filtering
impl WindowCollection {
    pub fn filter(&self, filters: WindowFilters) -> Vec<WindowHandle> {
        let state = self.state.lock().unwrap();
        state.windows
            .values()
            .filter(|window| window.matches_filters(&filters))
            .map(|window| WindowHandle::new(window.id, self.state.clone()))
            .collect()
    }
}
```

---

## 6. Security and Safety Architecture

### 6.1 Security Boundaries

**Safe Defaults**:
- No direct filesystem access (use `niri.action.spawn()` instead)
- No network access (require explicit opt-in)
- No system command execution without user confirmation
- Sandboxed file watching for config changes only

**Permission Model**:
```lua
-- Security-conscious API design
niri.config.enable("network_access")  -- Opt-in for network
niri.config.enable("filesystem_access")  -- Opt-in for file system
niri.config.set("allowed_spawn_commands", {"firefox", "alacritty"})
```

### 6.2 Error Handling Strategy

**Graceful Degradation**:
```lua
-- Protected Lua execution with error recovery
local success, result = pcall(function()
    niri.config.layout.gaps = "invalid"  -- Would error
end)

if not success then
    print("Configuration error:", result)
    -- Fallback to previous configuration
end
```

**Rust Error Propagation**:
```rust
// Comprehensive error handling in Lua API
impl LuaConfig {
    pub fn set_property(&mut self, path: &str, value: LuaValue) -> Result<(), ConfigError> {
        match path {
            "layout.gaps" => {
                let gaps: f64 = value.as_number()
                    .ok_or(ConfigError::InvalidType("Expected number for gaps"))?;
                if gaps < 0.0 || gaps > 100.0 {
                    return Err(ConfigError::InvalidValue("Gaps must be between 0 and 100"));
                }
                self.config.layout.gaps = gaps;
                Ok(())
            }
            _ => Err(ConfigError::UnknownProperty(path.to_string()))
        }
    }
}
```

---

## 7. Comparison with Other Systems

### 7.1 Architectural Comparison Matrix

| Feature | Niri (Spec) | Neovim | AwesomeWM | Pinnacle | WezTerm |
|---------|--------------|---------|------------|----------|---------|
| **Live State Access** | ✓ Yes | ✓ Yes | ✓ Yes | ✗ Snapshots | ✗ Config-only |
| **Hot Reload** | ✗ No (by design) | ✗ No (by design) | ✓ Full restart | ✗ Single-load | ✓ Auto-reload |
| **OO API** | ✓ Yes | ✗ Functions | ✓ Yes | ✗ Handles | ✗ Config |
| **Signal Events** | ✓ Yes | ✗ Autocmds | ✓ Yes | ✗ Callbacks | ✓ Events |
| **Type Safety** | ✓ Strong | ✓ Dynamic | ✓ Dynamic | ✓ Strong | ✓ Dynamic |
| **Performance** | ✓ Optimized | ✓ Fast | ✓ Fast | ✓ gRPC overhead | ✓ Fast |

### 7.2 Key Architectural Decisions

**Live State vs Snapshots**: 
- **Niri Choice**: Live state access like Neovim/AwesomeWM
- **Rationale**: Immediate feedback, essential for desktop environment
- **Implementation**: Direct state access via `StateHandle` with Arc<Mutex>

**No Hot Reload (Like Neovim)**:
- **Niri Choice**: No automatic hot reload, restart required for config changes
- **Rationale**: Predictability, safety, simpler implementation
- **Trade-off**: Minor inconvenience vs guaranteed consistency
- **Mitigation**: Fast startup, session restoration via Wayland protocols

**OO vs Functional API**:
- **Niri Choice**: Object-oriented like AwesomeWM
- **Rationale**: Intuitive window/workspace management
- **Implementation**: Rich Lua objects with methods

**Signal vs Callback Events**:
- **Niri Choice**: Signal-based like AwesomeWM
- **Rationale**: Decoupled communication, better for desktop environments
- **Implementation**: Event system with pattern matching

### 7.3 Hot Reload Comparison Deep Dive

| System | Approach | Pros | Cons |
|--------|----------|------|------|
| **AwesomeWM** | `awesome.restart()` - full process restart via `execvp()` | Clean slate, no memory leaks | Visual disruption, all Lua state lost |
| **Neovim** | No hot reload - manual `:source` only | Predictable, no hidden corruption | Developer friction |
| **Hammerspoon** | `hs.reload()` - fresh Lua environment | No visual disruption | All state lost unless explicitly saved |
| **WezTerm** | Auto-reload config files | Seamless updates | Limited to config, not scripting |
| **Niri (Spec)** | No hot reload (Neovim approach) | Predictable, safe, simple | Restart required for changes |

**Why Niri Follows Neovim**:

1. **Compositor Complexity**: Like Neovim's LSP/buffer state, compositor state (windows, workspaces, animations) is too complex for safe partial reload

2. **Closure Problem**: Lua closures capture references that become stale after reload
   ```lua
   local my_state = {}
   niri.event.on("window_open", function(win)
       my_state[win.id] = win  -- Captured reference corrupts after reload
   end)
   ```

3. **Predictability**: Users can always trust that restart = fresh state

4. **Implementation Simplicity**: No file watchers, no rollback logic, no state migration

---

## 8. Implementation Constraints

### 8.1 Technical Constraints

**Single-Threaded Lua**: All Lua execution on main compositor thread
**Luau Runtime**: Modern LuaJIT alternative with interrupt-based timeout protection
**Memory Limits**: Bounded memory usage per script execution
**Execution Limits**: Timeouts prevent infinite loops and resource exhaustion

### 8.2 Integration Constraints

**Non-Intrusive Design**: Lua runtime is optional, compositor works without it
**Clear Boundaries**: All interaction through defined APIs, no direct memory access
**Performance Isolation**: Lua errors don't crash compositor
**Resource Limits**: Bounded resource usage per script

### 8.3 Configuration Constraints

**KDL Compatibility**: KDL configuration remains foundation
**Gradual Migration**: Hybrid operation during transition period
**Backward Compatibility**: Existing KDL configurations continue to work
**Migration Path**: Clear upgrade path from KDL-only to Lua-enhanced

---

## 9. Success Criteria

### 9.1 Functional Requirements

**Complete API Coverage**: All Niri functionality accessible from Lua
**Live State Access**: Real-time state queries and manipulation
**Restart-Based Config**: Configuration changes via compositor restart (fast startup required)
**Performance Parity**: Lua operations within 10% of native Rust performance

### 9.2 Quality Requirements

**Type Safety**: Compile-time guarantees at Rust-Lua boundary
**Error Handling**: Graceful degradation and clear error messages
**Memory Safety**: No memory leaks or use-after-free errors
**Security**: Safe defaults with controlled opt-in for dangerous operations

### 9.3 User Experience Requirements

**Intuitive API**: Natural Lua ergonomics following system conventions
**Comprehensive Documentation**: Complete API reference with examples
**Debug Support**: Rich debugging and development tooling
**Community Ecosystem**: Support for third-party script distribution

---

## 10. Future Evolution Path

### 10.1 Phase 1: Foundation Implementation
- Core Lua runtime with Luau
- Basic configuration API (read/write)
- State query API with live access
- Simple action system
- Event system foundation
- Fast startup optimization (< 500ms target)

### 10.2 Phase 2: Advanced Features
- Advanced process management
- Timer and async primitives
- Comprehensive event coverage
- Performance optimizations
- Restart action for config reload workflow

### 10.3 Phase 3: Ecosystem Support
- Package management for Lua scripts
- Plugin system for third-party extensions
- Development tooling and debugging support
- Community script repository integration

---

## Conclusion

This specification defines a well-engineered Lua API architecture for Niri that combines the best patterns from successful Lua-embedded systems while making deliberate trade-offs for predictability and safety.

**Key Decisions**:
- **Live state access** for immediate feedback (like Neovim/AwesomeWM)
- **No hot reload** for predictable behavior (like Neovim, unlike AwesomeWM)
- **Object-oriented API** for intuitive desktop management (like AwesomeWM)
- **Signal-based events** for decoupled communication

The explicit choice to avoid hot reloading follows Neovim's philosophy: complex runtime state makes partial reload dangerous, and predictability is more valuable than convenience. Users restart the compositor for configuration changes, which is mitigated by fast startup times and Wayland session restoration.

This architecture provides a solid foundation for creating a highly extensible, scriptable desktop environment while maintaining the stability and predictability essential for a Wayland compositor.