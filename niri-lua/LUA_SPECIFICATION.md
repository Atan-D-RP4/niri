# Niri Lua Specification

Comprehensive specification for niri's Lua scripting system. This document covers the complete API surface, architecture, and implementation details.

**Current Version**: 25.11
**Minimum Supported**: 25.01
**Spec Revision**: 2.0 (optimized for agentic LLM consumption)

## Table of Contents

1. [Quick Reference](#quick-reference) *(single-page cheat sheet)*
2. [Overview](#overview)
3. [Vision](#vision)
4. [Design Decisions](#design-decisions)
5. [Architecture](#architecture)
6. [Configuration API](#configuration-api)
7. [Runtime State API](#runtime-state-api)
8. [Event System](#event-system)
9. [Action System](#action-system)
10. [Timer API](#timer-api)
11. [API Patterns](#api-patterns) *(decision tree for API usage)*
12. [Schema Reference](#schema-reference) *(machine-readable types)*
13. [Module System](#module-system)
14. [REPL](#repl)
15. [Appendices](#appendices)
    - [A: Canonical Reference Config](#appendix-a-canonical-reference-config)
    - [B: Version Compatibility Matrix](#appendix-b-version-compatibility-matrix)
    - [C: Test Coverage Matrix](#appendix-c-test-coverage-matrix)
    - [D: Code Review Checklist](#appendix-d-code-review-checklist)
    - [E: Refactor History](#appendix-e-refactor-history)

---

## Quick Reference

> **For LLM Agents**: This section provides a single-page overview of all APIs. Use this for quick lookups before diving into detailed sections.

### Namespaces

| Namespace | Purpose | Type |
|-----------|---------|------|
| `niri.config` | Configuration (KDL parity) | Table + Collections |
| `niri.state` | Runtime queries (read-only) | Table |
| `niri.events` | Event subscriptions | Methods |
| `niri.action` | Compositor actions | Methods |
| `niri.utils` | Logging and spawn utilities | Methods |
| `niri.loop` | Timer/scheduling | Methods |
| `niri.os` | Operating system utilities (hostname, env) | Methods |
| `niri.fs` | Filesystem utilities (which, expand, readable) | Methods |

### Config Collections (use `:add()`, `:remove()`, `:list()`, `:clear()`)

```lua
niri.config.binds:add(key, action)           -- Key/mouse/scroll bindings
niri.config.outputs:add({name, ...})         -- Monitor configuration
niri.config.window_rules:add({matches, ...}) -- Window rules
niri.config.layer_rules:add({matches, ...})  -- Layer surface rules
niri.config.workspaces:add({name, ...})      -- Named workspaces
niri.config.environment:add(key, value)      -- Environment variables
```

### Config Tables (use direct assignment)

```lua
niri.config.input = { keyboard = {...}, touchpad = {...}, ... }
niri.config.layout = { gaps = 16, focus_ring = {...}, ... }
niri.config.cursor = { xcursor_theme = "...", xcursor_size = 24 }
niri.config.animations = { slowdown = 1.0, ... }
niri.config.gestures = { workspace_swipe = {...} }
niri.config.overview = { backdrop_color = "..." }
niri.config.hotkey_overlay = { skip_at_startup = false }
niri.config.recent_windows = { ... }  -- Since 25.11
niri.config.clipboard = { disable_primary = false }
niri.config.switch_events = { lid_close = action, ... }
niri.config.debug = { ... }
```

### Config Scalars (use direct assignment)

```lua
niri.config.prefer_no_csd = true
niri.config.screenshot_path = "~/Pictures/%Y-%m-%d_%H-%M-%S.png"
```

### Runtime State Queries

```lua
local windows = niri.state.windows()        -- {id, app_id, title, workspace_id, ...}[]
local workspaces = niri.state.workspaces()  -- {id, name, output, is_active, ...}[]
local outputs = niri.state.outputs()        -- {name, make, model, width, height, ...}[]
local layouts = niri.state.keyboard_layouts() -- {names, current_idx}
local focused = niri.state.focused_window() -- {id, app_id, title} | nil
local cursor = niri.state.cursor_position() -- {x, y, output} | nil (Since 25.XX)
local reserved = niri.state.reserved_space("eDP-1") -- {top, bottom, left, right} (Since 25.XX)
local mode = niri.state.focus_mode()        -- "normal"|"overview"|"layer_shell"|"locked" (Since 25.XX)
```

### Events (28 available, 4 excluded for security)

```lua
-- Single event subscription
niri.events:on("window:open", function(ev) end)    -- ev: {id, app_id, title, workspace_id}
niri.events:on("window:close", function(ev) end)   -- ev: {id, app_id, title}
niri.events:on("window:focus", function(ev) end)   -- ev: {id, app_id, title}
niri.events:on("workspace:activate", function(ev) end)  -- ev: {name, idx, output}
niri.events:on("monitor:connect", function(ev) end)     -- ev: {name, connector}
niri.events:on("config:reload", function(ev) end)       -- ev: {}
niri.events:on("overview:open", function(ev) end)       -- ev: {}

-- Multi-event subscription (vim-style)
niri.events:on({"window:open", "window:close"}, function(ev) end)  -- Same callback for multiple events
```

### Common Actions

```lua
-- Window
niri.action:close_window()
niri.action:fullscreen_window()
niri.action:focus_window(id)              -- id: integer
niri.action:focus_window_up()
niri.action:focus_window_down()

-- Workspace
niri.action:focus_workspace(ref)          -- ref: integer | string
niri.action:move_window_to_workspace(ref)

-- Spawn
niri.action:spawn({"cmd", "arg1"})        -- array of strings
niri.action:spawn_sh("shell command")     -- shell string

-- Layout
niri.action:toggle_window_floating()
niri.action:toggle_column_tabbed()
niri.action:set_column_width({proportion = 0.5})

-- System
niri.action:quit()
niri.action:power_off_monitors()
niri.action:screenshot()
```

### Timers

```lua
local timer = niri.loop.new_timer()
timer:start(1000, 0, function() end)  -- one-shot after 1000ms
-- For repeating:
timer:start(1000, 1000, function() end)  -- repeat every 1000ms
timer:close()  -- cancel
```

### Logging and Spawning

```lua
niri.utils.log("info message")
niri.utils.debug("debug message")
niri.utils.warn("warning message")
niri.utils.error("error message")

-- Fire-and-forget spawn
niri.utils.spawn({"notify-send", "Hello"})
```

---

## Overview

The niri Lua system enables configuration and runtime scripting of the niri Wayland compositor. Built on the [mlua](https://github.com/mlua-rs/mlua) crate with the **Luau** dialect (not LuaJIT), it provides timeout protection, type safety, and a comprehensive API for:

- **Configuration**: Full KDL config parity via Lua tables
- **Runtime queries**: Access window, workspace, and output state
- **Event handling**: Subscribe to compositor events
- **Actions**: Execute all niri actions programmatically
- **Timers**: Schedule deferred and repeating callbacks

### Key Design Principles

1. **Safety first**: Luau's `set_interrupt` provides reliable timeout protection (default 1 second)
2. **No deadlocks**: Event context snapshots avoid lock contention during callbacks
3. **Composable API**: Each namespace (`niri.config`, `niri.events`, etc.) is independent
4. **KDL parity**: Every KDL config option has a Lua equivalent

### Implementation Stats

- ~8,000 lines of Rust across 26 modules
- 400+ passing tests
- Full KDL configuration parity (all sections implemented)
- ~90 actions available
- 32 events implemented (4 intentionally excluded for security: idle:*, key:*)

---

## Vision

Niri's Lua system is foundational infrastructure for transforming niri from a standalone compositor into a **complete desktop environment framework**—similar to how Neovim serves as a base that becomes a full IDE through Lua configuration (LazyVim, LunarVim, AstroNvim).

### The Desktop Environment Framework

The long-term vision comprises three complementary components:

| Component | Purpose | Status | Documentation |
|-----------|---------|--------|---------------|
| **niri-lua** | Configuration, runtime state, events, actions | ✅ Implemented | This document |
| **OS Utilities** | `niri.os.*` and `niri.fs.*` APIs for system access | 📋 Specified | [NIRI_LUA_OS_UTILITIES_SPEC.md](../docs/NIRI_LUA_OS_UTILITIES_SPEC.md) |
| **niri-ui** | Smithay-native widget toolkit for shell components | 📋 Design phase | [NIRI_UI_SPECIFICATION.md](../docs/NIRI_UI_SPECIFICATION.md) |

Together, these enable building full desktop shells comparable to:
- **AwesomeWM**: Lua-configured window manager with built-in widgets
- **QtQuick/Shell projects**: Noctalia Shell, DankMaterialShell
- **KDE Plasma**: Full desktop environment with panels, widgets, system integration

### Comparison with Lua-Configured Projects

| Project | Type | Lua Role | Widgets | Wayland | Key Difference |
|---------|------|----------|---------|---------|----------------|
| **niri-lua** | Compositor | Config + runtime | Via niri-ui | Native | Scrollable tiling, Smithay-based |
| **AwesomeWM** | Window Manager | Config + widgets | Built-in | ❌ X11 | Mature ecosystem, X11-only |
| **Pinnacle** | Compositor | Config + runtime | Via Snowcap (gRPC) | Native | Separate UI process, Iced-based |
| **WezTerm** | Terminal | Config only | N/A | N/A | Similar API patterns (inspiration) |
| **Neovim** | Editor | Full scripting | Plugins | N/A | Plugin ecosystem model (inspiration) |

#### AwesomeWM Comparison

AwesomeWM pioneered the "Lua-configured desktop" paradigm. Key lessons:

| AwesomeWM | niri Equivalent | Notes |
|-----------|-----------------|-------|
| `rc.lua` | `niri.lua` | Main configuration entry point |
| `awful.spawn` | `niri.action:spawn()` | Process launching |
| `gears.filesystem.*` | `niri.fs.*` | Path and file utilities |
| `awful.widget.*` | `niri.ui.*` (planned) | Built-in widget library |
| `naughty` | `niri.notifications` (planned) | Notification system |
| `client.connect_signal()` | `niri.events:connect()` | Event subscription |

**Key architectural difference**: AwesomeWM is X11-only with a synchronous Lua model. Niri is Wayland-native with compositor-integrated widgets and timeout-protected Lua execution.

#### Pinnacle/Snowcap Comparison

Pinnacle (Smithay-based, like niri) uses a **separate UI process** (Snowcap) communicating via gRPC:

| Aspect | Pinnacle/Snowcap | niri/niri-ui |
|--------|------------------|--------------|
| **Process model** | Separate processes | Single process |
| **Communication** | gRPC over Unix socket | Direct function calls |
| **Widget framework** | Iced (external) | Smithay-native (internal) |
| **State access** | Serialized over IPC | Direct shared state |
| **Crash isolation** | UI crash doesn't affect compositor | UI rendered safely via textures |

**Trade-off**: Pinnacle's approach provides stronger crash isolation but adds IPC latency and complexity. Niri's integrated approach enables lower latency and simpler deployment while maintaining stability through compositor-internal rendering (like `hotkey_overlay`).

### What niri-lua Provides Today

1. **Complete Configuration**: Full KDL parity—every config option is scriptable
2. **Runtime State Queries**: Access to windows, workspaces, outputs, keyboard layouts
3. **Event System**: 28 compositor events for reactive programming
4. **Action System**: ~90 actions for controlling the compositor
5. **Timers**: Deferred and repeating callbacks for dynamic behavior
6. **REPL**: Interactive Lua console for debugging and exploration

### What OS Utilities Will Add

The `niri.os` and `niri.fs` APIs (see [NIRI_LUA_OS_UTILITIES_SPEC.md](../docs/NIRI_LUA_OS_UTILITIES_SPEC.md)) provide simple, secure OS and filesystem utilities useful during configuration time.

- `niri.os.hostname()` → `string`
  - Returns the system hostname as UTF-8 string
  - Throws only on invalid UTF-8 (message starts with `niri.os.hostname:`)
  - On non-UTF8 system errors, returns empty string (no throw)

- `niri.os.getenv(name)` → `string | nil`
  - Returns the environment variable value or `nil` if unset
  - Returns empty string `""` if variable is set to an empty string

- `niri.fs.which(cmd)` → `string | nil`
  - Finds an executable in PATH (respecting `PATHEXT` on Windows)
  - If `cmd` contains a path separator or is absolute, treats as path and returns it if executable
  - Returns `nil` for missing or non-executable files
  - Returns `nil` for empty string argument

- `niri.fs.readable(path)` → `boolean`
  - Returns `true` if the file exists and is readable
  - Follows symlinks; broken symlinks return `false`
  - Never throws on bad paths or IO errors

- `niri.fs.expand(path)` → `string`
  - Expands `~`, `$VAR`, and `${VAR}` in paths (Neovim-aligned semantics)
  - Unset environment variables expand to empty string (e.g., `$UNSET/path` -> `/path`)
  - On failure to expand, returns the original path unchanged

Examples:
```lua
-- Conditional xwayland-satellite
if niri.fs.which("xwayland-satellite") then
    niri.config.xwayland_satellite.off = false
end

-- Path expansion and env var usage
local config_path = niri.fs.expand("$XDG_CONFIG_HOME/niri/init.lua")
local home = niri.os.getenv("HOME")
```

**Notes:**
- All `niri.fs` check functions never throw; they are designed to be safe during config-time evaluation.
- No caching is performed; functions re-check each call (config-time usage only).
### What niri-ui Will Add

The planned `niri-ui` crate (see [NIRI_UI_SPECIFICATION.md](../docs/NIRI_UI_SPECIFICATION.md)) will enable:

- **Status bars and panels**: Top/bottom bars with workspaces, clock, system info
- **Application launchers**: dmenu/rofi-style launchers built in Lua
- **Notification centers**: OSD and notification management
- **System trays**: SNI (StatusNotifierItem) protocol support
- **Custom overlays**: Volume/brightness indicators, window switchers
- **D-Bus integration**: Full system service access from Lua

### The Neovim Analogy

| Neovim | Niri | Notes |
|--------|------|-------|
| Base editor | Base compositor | Core functionality |
| `vim.api.*` | niri-lua | Configuration and runtime API |
| `vim.fn.*`, `vim.loop.*` | `niri.os.*`, `niri.fs.*` | OS/filesystem utilities |
| UI plugins (telescope, lualine) | niri-ui widgets | Visual components |
| Distribution (LazyVim) | Community "niri distros" | Curated configurations |

Just as Neovim users can build entirely custom editing experiences through Lua, niri users will be able to build entirely custom desktop experiences—without writing any Rust.

### Design Principles for the Framework

1. **Compositor stability is paramount**: UI crashes must never take down the compositor
2. **Smithay-native rendering**: No external GUI dependencies; use niri's existing Cairo/Pango → GlowRenderer pipeline
3. **Minimal compositor changes**: niri-ui lives in a separate crate with <50 lines of integration code
4. **Lua-first API**: All shell components are defined declaratively in Lua tables
5. **Simple plugin model**: Neovim-style `require()` from extended `package.path`

### Current Status

- ✅ **niri-lua**: Complete and production-ready
  - Configuration API: Full KDL parity (input, outputs, layout, binds, window_rules, etc.)
  - Runtime API: Windows, workspaces, outputs, keyboard layouts
  - Events: 28 compositor events (Window, Workspace, Layout, System, Monitor, Output, Lock, Lifecycle)
  - Actions: ~90 actions
  - REPL: Interactive development

- ✅ **Compositor Integration**: Complete
  - `src/lua_integration.rs`: Consolidated Lua setup (~12 lines in main.rs)
  - `src/lua_event_hooks.rs`: Extension traits for event emission
  - Centralized event emission in refresh cycle (not scattered call sites)

- 📋 **OS Utilities**: Specified
  - `niri.os.hostname()`, `niri.os.getenv()`
  - `niri.fs.which()`, `niri.fs.readable()`, `niri.fs.expand()`
  - See [NIRI_LUA_OS_UTILITIES_SPEC.md](../docs/NIRI_LUA_OS_UTILITIES_SPEC.md)

- 📋 **niri-ui**: Design phase
  - Specification complete
  - Architecture designed
  - Awaiting implementation

---

## Design Decisions

This section documents the key architectural decisions made during implementation and the rationale behind each choice.

### Lua Runtime: Luau over LuaJIT

**Decision:** Use Luau (Roblox's Lua dialect) instead of LuaJIT.

**Rationale:** Reliable timeout protection is critical for a compositor. LuaJIT's debug hooks don't fire when the JIT compiler is active, making timeouts unreliable without either:
- Disabling JIT around user code (10-20% performance hit), or
- Using unsafe signal handlers with `pthread_kill` (~150 LOC of unsafe code)

Luau's `set_interrupt` callback fires periodically during execution, even in optimized code, enabling clean wall-clock timeout protection without unsafe code.

**Trade-offs:**
- Luau is based on Lua 5.1 (same as LuaJIT/Neovim), so most code is compatible
- Some LuaJIT-specific extensions (FFI) are not available
- Luau-specific features (type annotations, `continue`, compound assignment) aren't portable to other Lua environments

```toml
# Cargo.toml
mlua = { version = "0.11.4", features = ["luau", "vendored"] }
```

### Timeout Protection: 1 Second Default

**Decision:** Default timeout of 1 second for all Lua execution.

**Rationale:**
- Long enough for complex configuration logic
- Short enough to catch infinite loops before user notices freeze
- Configurable for trusted code (`ExecutionLimits::unlimited()`)

**Comparison with alternatives:**

| Scenario | What Happens |
|----------|--------------|
| `while true do end` in config | Script times out after 1 second, error reported |
| `while true do end` in event handler | Callback times out, compositor continues |
| `while true do end` in REPL | Command times out, REPL remains usable |

**This is a major improvement over Neovim/AwesomeWM** which have no timeout protection.

### Shared State: std::sync::Mutex over parking_lot

**Decision:** Use `std::sync::Mutex` instead of `parking_lot::Mutex`.

**Rationale:**
- The Lua runtime is single-threaded, but we need re-entrancy safety for nested event emission
- `parking_lot` is overkill for single-threaded Lua execution
- Removes an unnecessary dependency
- `Arc` still needed for shared ownership across closures

```rust
// Allows nested event emission without deadlock
let handlers = std::sync::Mutex<EventHandlers>;
```

### Thread Safety: mlua `send` Feature Not Used

**Decision:** Don't use mlua's `send` feature that makes Lua values `Send + Sync`.

**Rationale:**
- `LuaFunction` is not `Send + Sync` without the feature
- Enabling `send` incurs performance overhead for cross-thread safety we don't need
- All Lua execution happens on the main compositor thread
- `Arc<Mutex<T>>` is sufficient for shared state

### Callback Scheduling: Hybrid Flush with Limit

**Decision:** Process at most 16 scheduled callbacks per flush cycle, with max 1000 queue size.

**Rationale:**
- Prevents callback chains from starving the compositor (bounds latency)
- Allows some chaining within a cycle for common patterns
- Queue limit prevents unbounded memory growth from runaway scheduling

```rust
// Module-level constants (not struct fields)
const MAX_CALLBACKS_PER_FLUSH: usize = 16;
const MAX_QUEUE_SIZE: usize = 1000;

pub struct ExecutionLimits {
    pub timeout: Duration,  // Default: 1 second
}
```

### Timer Lifetime: Persist Until Explicit close()

**Decision:** Timers continue running even if the Lua handle is garbage collected.

**Rationale:**
- Matches Neovim's `vim.uv` semantics (familiar to users)
- Explicit resource management is clearer than implicit GC-based cleanup
- Prevents unexpected timer cancellation when handles go out of scope

```lua
-- Timer continues even without handle reference
niri.loop.new_timer():start(1000, 0, function()
    print("Still fires!")  -- GC doesn't stop this
end)

-- Proper cleanup pattern
local timer = niri.loop.new_timer()
timer:start(1000, 0, function()
    timer:close()  -- Explicit cleanup
end)
```

### Event Context: Snapshots over Live References

**Decision:** Event handlers receive snapshots (copies) of state, not live references.

**Rationale:**
- Avoids deadlocks from callbacks trying to acquire locks already held
- Prevents race conditions if compositor state changes during callback
- Slightly higher memory usage but much safer

```rust
// In runtime_api.rs
fn create_state_snapshot(&self) -> StateSnapshot {
    // Copy state while holding lock briefly
    // Release lock before calling Lua
}
```

### Config Conversion: Direct UserData Proxies

**Decision:** Use native `mlua::UserData` proxy structs for direct Lua-to-Config conversion.

**Rationale:**
- No serialization overhead (direct field access)
- Type-safe field assignments with immediate validation
- Better error messages (field-level rather than post-serialization)
- Enables lazy/reactive patterns (only modified fields trigger updates)

**Implementation:**
```rust
// Each config section has a dedicated proxy struct
impl UserData for LayoutProxy {
    fn add_fields<F: LuaUserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("gaps", |_, this| {
            Ok(this.config.lock().unwrap().layout.gaps)
        });
        fields.add_field_method_set("gaps", |_, this, value: u16| {
            this.config.lock().unwrap().layout.gaps = value;
            this.dirty.lock().unwrap().layout = true;
            Ok(())
        });
    }
}
```

**Historical note:** An earlier design considered `serde_json::Value` as an intermediary, but this was replaced with direct UserData proxies for better performance and type safety.

### Type Definitions: EmmyLua over Luau Native Types

**Decision:** Use EmmyLua annotations (`---@class`, `---@param`) instead of Luau's native type syntax.

**Rationale:**
- Compatible with emmylua-analyzer-rust, lua_ls, and other common LSPs
- Users may not have Luau-aware LSPs configured
- Generates `types/api.lua` usable with any Lua LSP

### Event Selection: No Raw Key or Idle Events

**Decision:** Don't expose raw keyboard events or idle events to Lua.

**Rationale:**

| Event Type | Why Excluded |
|------------|--------------|
| `key:press`, `key:release` | Security (keylogging potential), performance (every keystroke), not needed (keybindings cover use cases) |
| `idle:start`, `idle:end` | Not exposed via Smithay's IdleNotifierState, better controlled via config |

AwesomeWM also uses a keybinding registration model rather than raw key events.

### Compiler Optimization: Level 2 with Debug Info

**Decision:** Use Luau compiler optimization level 2 with debug level 1.

**Rationale:**
- Level 2 enables: function inlining, loop unrolling, constant folding, dead code elimination
- Debug level 1 preserves: line numbers in error messages, function names for stack traces

```rust
let compiler = Compiler::new()
    .set_optimization_level(2)  // Aggressive optimizations
    .set_debug_level(1);        // Keep line info for errors
```

### Summary Table

| Decision | Choice | Alternative Rejected | Rationale |
|----------|--------|---------------------|-----------|
| Lua runtime | Luau | LuaJIT | Reliable `set_interrupt` for timeouts |
| Timeout | 1 second | Unlimited | Catch infinite loops, configurable |
| Mutex | std::sync | parking_lot | Simpler, no extra dependency |
| Thread safety | Single-threaded | mlua `send` | Performance, not needed |
| Callback flush | Limit 16/cycle | Unlimited | Bound latency |
| Timer lifetime | Until close() | GC-based | Match Neovim, explicit is clearer |
| Event state | Snapshots | Live refs | Avoid deadlocks |
| Config convert | UserData proxies | JSON intermediary | Better performance, type safety |
| Type defs | EmmyLua | Luau native | LSP compatibility |
| Raw keys | Excluded | Included | Security, performance |

---

## Architecture

### Tiered Implementation

The Lua system is organized into tiers of increasing functionality:

| Tier | Feature | Status |
|------|---------|--------|
| 1 | Module system, event emitter | Complete |
| 2 | Configuration API with full KDL parity | Complete |
| 3 | Runtime state queries (windows, workspaces, outputs) | Complete |
| 4 | Event system with compositor integration | Complete |
| 5 | Module loader (XDG paths defined, integration pending) | Partial |
| 6 | Developer experience (REPL, docs, types) | Complete |

### Core Components

```
niri-lua/src/
├── lib.rs              # Public API exports
├── runtime.rs          # LuaRuntime core (timeout, scheduler, timer management)
├── config.rs           # Config struct definitions
├── config_api.rs       # niri.config API entry point
├── config_wrapper.rs   # Config proxies with section access (~1,550 LOC)
├── config_proxies.rs   # Derive macro-based proxies (~1,650 LOC)
├── config_dirty.rs     # Dirty tracking for config changes
├── action_proxy.rs     # niri.action API (~90 actions via macro)
├── events_proxy.rs     # niri.events API (on, once, off, emit, list, clear)
├── event_system.rs     # Core event infrastructure
├── event_data.rs       # Event data enums (Window, Workspace, Monitor, Layout)
├── event_handlers.rs   # Event handler management
├── runtime_api.rs      # niri.state API (state snapshots)
├── niri_api.rs         # Main niri.* namespace
├── loop_api.rs         # niri.loop timer API (Neovim-style)
├── ipc_repl.rs         # Interactive REPL
├── ipc_bridge.rs       # IPC communication bridge
├── api_registry.rs     # API namespace registration
├── api_data.rs         # Shared API schema definitions
├── lua_api_schema.rs   # Lua API type schema
├── lua_types.rs        # Lua type utilities
├── collections.rs      # Lua collection utilities
├── extractors.rs       # Value extraction helpers
├── parse_utils.rs      # Parsing utilities
├── test_utils.rs       # Test helpers
└── types/api.lua       # EmmyLua type definitions (generated)

src/
├── lua_integration.rs  # Compositor-side Lua setup (consolidates main.rs logic)
└── lua_event_hooks.rs  # Extension traits for centralized event emission
```

### Compositor Integration Module (`src/lua_integration.rs`)

The `lua_integration` module consolidates all Lua setup logic from `main.rs` into reusable functions:

| Function | Purpose |
|----------|---------|
| `load_lua_config()` | Loads Lua config file, applies to Config (with dirty flag check) |
| `create_action_channel()` | Creates calloop channel for Lua actions (with `advance_animations()`) |
| `setup_runtime()` | Registers RuntimeApi, ConfigWrapper, and ActionProxy |
| `execute_pending_actions()` | Runs deferred actions from config load |
| `is_lua_config_active()` | Checks if Lua runtime is present |

This reduces ~150 lines of Lua code in `main.rs` to ~12 lines of function calls.

### LuaRuntime

The central `LuaRuntime` struct manages:

```rust
pub struct LuaRuntime {
    lua: Lua,                           // mlua instance (Luau)
    config_wrapper: ConfigWrapper,      // Configuration state
    event_system: EventSystem,          // Event subscriptions
    timer_manager: TimerManager,        // Active timers
    scheduled_callbacks: Vec<...>,      // Deferred callbacks
    execution_limits: ExecutionLimits,  // Timeout settings
}
```

### Execution Limits

```rust
// Module-level constants for scheduling limits
const MAX_CALLBACKS_PER_FLUSH: usize = 16;
const MAX_QUEUE_SIZE: usize = 1000;

pub struct ExecutionLimits {
    pub timeout: Duration,  // Default: 1 second
}

impl ExecutionLimits {
    pub fn default() -> Self {
        Self { timeout: Duration::from_secs(1) }
    }

    pub fn unlimited() -> Self {
        Self { timeout: Duration::MAX }
    }
}
```

---

## Configuration API

The `niri.config` namespace provides complete KDL configuration parity. All niri configuration options are accessible via Lua tables and collection methods.

### Architecture

- **Source files**: `config_wrapper.rs` (~3,200 LOC), `config_api.rs` (~950 LOC)
- **Pattern**: Hybrid table/collection proxies with dirty tracking
- **Conversion**: Lua → JSON → Config structs (via serde)

### Configuration Categories

| Category | Type | Access Pattern |
|----------|------|----------------|
| `input` | table | `niri.config.input.keyboard.xkb.layout = "us"` |
| `layout` | table | `niri.config.layout.gaps = 16` |
| `cursor` | table | `niri.config.cursor.xcursor_size = 24` |
| `animations` | table | `niri.config.animations.slowdown = 2.0` |
| `binds` | collection | `niri.config.binds:add("Mod+Q", action)` |
| `outputs` | collection | `niri.config.outputs:add({name = "eDP-1", ...})` |
| `window_rules` | collection | `niri.config.window_rules:add({matches = {...}})` |
| `layer_rules` | collection | `niri.config.layer_rules:add({...})` |
| `workspaces` | collection | `niri.config.workspaces:add({name = "dev"})` |
| `environment` | collection | `niri.config.environment:add("KEY", "value")` |

### Table Fields (Direct Assignment)

```lua
-- Nested table access
niri.config.input.keyboard.xkb.layout = "us"
niri.config.input.touchpad.tap = true
niri.config.layout.gaps = 16
niri.config.layout.focus_ring.width = 4
niri.config.layout.focus_ring.active_color = "#7fc8ff"

-- Full table replacement
niri.config.cursor = {
    xcursor_theme = "Adwaita",
    xcursor_size = 24,
    hide_when_typing = true,
}

-- Scalar fields
niri.config.prefer_no_csd = true
niri.config.screenshot_path = "~/Pictures/%Y-%m-%d_%H-%M-%S.png"
```

### Collection Fields (Method Access)

Collections use method-based CRUD operations:

```lua
-- Add items
niri.config.binds:add("Mod+Q", niri.action:close_window())
niri.config.binds:add("Mod+Return", {
    action = niri.action:spawn({"alacritty"}),
    allow_when_locked = false,
})

-- List all items
local all_binds = niri.config.binds:list()

-- Get specific item
local bind = niri.config.binds:get("Mod+Q")

-- Remove item
niri.config.binds:remove("Mod+Q")

-- Clear all
niri.config.binds:clear()

-- Replace entire collection
niri.config.binds:set({
    ["Mod+Q"] = niri.action:close_window(),
    ["Mod+Return"] = niri.action:spawn({"alacritty"}),
})
```

### Batch Updates

For performance when making multiple changes:

```lua
niri.config:update({
    layout = {
        gaps = 20,
        focus_ring = { width = 6 },
    },
    cursor = {
        xcursor_size = 32,
    },
})
```

### Applying Configuration

After making changes, apply them to the compositor:

```lua
niri.config:apply()  -- Applies all pending changes
```

---

## Runtime State API

The `niri.state` namespace provides read-only access to compositor runtime state including windows, workspaces, outputs, and keyboard layouts.

### Architecture

- **Source file**: `runtime_api.rs` (~1185 LOC)
- **Pattern**: Snapshot-based queries to avoid lock contention
- **Dual mode**: Event handlers use pre-populated snapshots; normal code uses idle callbacks

### Core Components

```rust
// Internal structures (from runtime_api.rs)
pub struct StateSnapshot {
    pub windows: Vec<WindowData>,
    pub workspaces: Vec<WorkspaceData>,
    pub outputs: Vec<OutputData>,
    pub focused_window: Option<WindowData>,
    pub cursor_position: Option<CursorPosition>,  // Since 25.XX
    pub focus_mode: FocusMode,                    // Since 25.XX
}

// Thread-local for event context
thread_local! {
    static EVENT_CONTEXT_STATE: RefCell<Option<StateSnapshot>> = RefCell::new(None);
}
```

### State Queries

```lua
-- Get all windows
local windows = niri.state.windows()
-- Returns: { {id, app_id, title, workspace_id, is_focused, is_floating, ...}, ... }

-- Get all workspaces
local workspaces = niri.state.workspaces()
-- Returns: { {id, name, output, is_active, idx, ...}, ... }

-- Get all outputs
local outputs = niri.state.outputs()
-- Returns: { {name, make, model, width, height, refresh, scale, x, y, transform, ...}, ... }

-- Get keyboard layouts
local layouts = niri.state.keyboard_layouts()
-- Returns: { names = {"us", "de"}, current_idx = 0 }

-- Get focused window (may be nil)
local focused = niri.state.focused_window()
-- Returns: {id, app_id, title} or nil

-- Get cursor position (Since 25.XX)
local cursor = niri.state.cursor_position()
-- Returns: {x = number, y = number, output = string} | nil
-- Returns nil when: no pointing device, during pointer grab, or position undefined

-- Get reserved space from layer-shell exclusive zones (Since 25.XX)
local reserved = niri.state.reserved_space("eDP-1")
-- Returns: {top = number, bottom = number, left = number, right = number}
-- Returns zeros for invalid output or no exclusive zones

-- Get current compositor focus mode (Since 25.XX)
local mode = niri.state.focus_mode()
-- Returns: "normal" | "overview" | "layer_shell" | "locked"
```

### New State API Functions (Since 25.XX)

#### cursor_position()

Returns the current cursor position in global compositor coordinates.

**Returns**: `{x: number, y: number, output: string} | nil`

| Field | Type | Description |
|-------|------|-------------|
| `x` | number | X coordinate in global space (pixels) |
| `y` | number | Y coordinate in global space (pixels) |
| `output` | string | Name of output where cursor is located |

**Returns `nil` when**:
- No pointing device is connected
- Cursor is grabbed by a surface (e.g., during window resize or move)
- Compositor is in a state where cursor position is undefined

**Example**:
```lua
-- Position popup at cursor
local cursor = niri.state.cursor_position()
if cursor then
    print(string.format("Cursor at (%d, %d) on %s", cursor.x, cursor.y, cursor.output))
end

-- Check if cursor is on specific output
niri.events:on("pointer:move", function()
    local pos = niri.state.cursor_position()
    if pos and pos.output == "eDP-1" then
        -- Cursor is on laptop display
    end
end)
```

#### reserved_space(output_name)

Returns the space reserved by layer-shell surfaces (panels, docks) with exclusive zones on a specific output.

**Parameters**:
| Parameter | Type | Description |
|-----------|------|-------------|
| `output_name` | string | Name of the output to query (e.g., "eDP-1") |

**Returns**: `{top: number, bottom: number, left: number, right: number}`

| Field | Type | Description |
|-------|------|-------------|
| `top` | number | Pixels reserved at top edge |
| `bottom` | number | Pixels reserved at bottom edge |
| `left` | number | Pixels reserved at left edge |
| `right` | number | Pixels reserved at right edge |

**Returns `{top=0, bottom=0, left=0, right=0}` when**:
- Output doesn't exist
- No layer-shell surfaces have exclusive zones on this output

**Note**: When multiple surfaces anchor to the same edge, this returns the **maximum** exclusive zone value (not the sum). For example, if two panels anchor to the top with exclusive_zone=32 and exclusive_zone=48, `reserved_space().top` returns 48.

**Example**:
```lua
-- Avoid overlapping with existing panels
local function create_bottom_panel(output_name)
    local reserved = niri.state.reserved_space(output_name)
    
    if reserved.bottom > 0 then
        niri.utils.warn("Bottom edge already has " .. reserved.bottom .. "px reserved")
        return
    end
    
    -- Safe to create panel on bottom edge
end

-- Calculate usable area for window placement
local function get_usable_area(output_name)
    local outputs = niri.state.outputs()
    local output = nil
    for _, o in ipairs(outputs) do
        if o.name == output_name then
            output = o
            break
        end
    end
    
    if not output then return nil end
    
    local reserved = niri.state.reserved_space(output_name)
    
    return {
        x = output.x + reserved.left,
        y = output.y + reserved.top,
        width = output.width - reserved.left - reserved.right,
        height = output.height - reserved.top - reserved.bottom,
    }
end
```

#### focus_mode()

Returns the current focus mode of the compositor, indicating which type of surface or UI has keyboard focus.

**Returns**: `string` - One of:

| Value | Description |
|-------|-------------|
| `"normal"` | Normal window focus mode (includes Layout, Mru, ScreenshotUi, ExitConfirmDialog) |
| `"overview"` | Overview mode is active |
| `"layer_shell"` | A layer-shell surface has keyboard focus |
| `"locked"` | Screen is locked (highest priority) |

**Mode Priority**: When multiple conditions exist, the following priority order applies:
1. `"locked"` - Screen lock takes absolute precedence
2. `"overview"` - Overview mode supersedes normal focus
3. `"layer_shell"` - Layer-shell surface has focus
4. `"normal"` - Default state (includes transient UI states)

**Note**: Several KeyboardFocus variants map to `"normal"` because they represent transient UI states rather than persistent mode changes:
- `Layout` - normal window focus
- `Mru` - MRU window switcher (transient overlay)
- `ScreenshotUi` - screenshot selection UI (transient)
- `ExitConfirmDialog` - exit confirmation dialog (transient)

**Example**:
```lua
-- Conditional UI based on focus mode
niri.events:on("overview:open", function()
    if niri.state.focus_mode() == "overview" then
        -- Hide elements that would conflict with overview
    end
end)

-- Prevent popup in certain modes
local function show_context_menu()
    local mode = niri.state.focus_mode()
    if mode == "locked" or mode == "overview" then
        return  -- Don't show menu in these modes
    end
    
    -- Safe to show menu
end

-- Adjust UI appearance based on mode
local function get_panel_opacity()
    local mode = niri.state.focus_mode()
    if mode == "overview" then
        return 0.5  -- Dim panel in overview
    elseif mode == "locked" then
        return 0.0  -- Hide panel when locked
    else
        return 1.0
    end
end
```

### Event Context Behavior

Inside event handlers, state queries return a snapshot taken at event emission time:

```lua
niri.events:on("window:open", function(ev)
    -- State is consistent with when the event was emitted
    local windows = niri.state.windows
    -- This is a snapshot, not live data
end)
```

Outside event handlers, state queries schedule idle callbacks to safely access compositor state.

---

## Event System

The event system enables reactive programming by subscribing to compositor events.

### Architecture

- **Source files**: `events_proxy.rs` (790 LOC), `event_data.rs` (200+ LOC), `event_handlers.rs`
- **Pattern**: Colon-method syntax (`:on()`, `:off()`, etc.)
- **COW optimization**: `Arc<Vec>` for handlers to avoid cloning during iteration

### EventsProxy Methods

| Method | Signature | Description |
|--------|-----------|-------------|
| `on` | `:on(event, callback)` | Subscribe to event(s), returns handler ID(s) |
| `once` | `:once(event, callback)` | Subscribe once, auto-removes after first fire |
| `off` | `:off(event, handler_id)` | Unsubscribe specific handler |
| `emit` | `:emit(event, data)` | Manually emit event (for testing) |
| `list` | `:list()` | List all registered event names |
| `clear` | `:clear()` | Remove all handlers |

### Subscription Examples

```lua
-- Single event subscription
local handler_id = niri.events:on("window:open", function(ev)
    print("Window opened: " .. ev.app_id)
end)

-- Multi-event subscription (same callback)
local handler_ids = niri.events:on({"window:open", "window:close"}, function(ev)
    print("Window event: " .. ev.event_type)
end)

-- One-time subscription
niri.events:once("config:reload", function(ev)
    print("Config reloaded!")
end)

-- Unsubscribe
niri.events:off("window:open", handler_id)
niri.events:off(handler_ids)  -- Multi-event unsubscribe
```

### Return Value Semantics

| Subscription Type | Return Value |
|-------------------|--------------|
| Single event `:on("event", fn)` | `integer` (handler ID) |
| Multi-event `:on({"e1", "e2"}, fn)` | `{["e1"] = id1, ["e2"] = id2}` |
| Single event `:once("event", fn)` | `integer` (handler ID) |
| Multi-event `:once({"e1", "e2"}, fn)` | `{["e1"] = id1, ["e2"] = id2}` |

### off() Variants

| Signature | Behavior | Return |
|-----------|----------|--------|
| `:off("event", id)` | Remove specific handler | `boolean` |
| `:off("event")` | Remove ALL handlers for event | `boolean` |
| `:off(handler_table)` | Remove multiple handlers | `{[event] = boolean, ...}` |

### emit() Data Wrapping

The `emit()` method normalizes data before passing to handlers:

| Input Type | Handler Receives |
|------------|------------------|
| Table `{foo = 1}` | `{foo = 1}` (unchanged) |
| Primitive `42` | `{value = 42}` |
| Primitive `"hello"` | `{value = "hello"}` |
| Primitive `true` | `{value = true}` |
| `nil` | `{}` (empty table) |

```lua
-- Examples
niri.events:emit("custom:event", {foo = "bar"})  -- Handler sees {foo = "bar"}
niri.events:emit("custom:event", 42)             -- Handler sees {value = 42}
niri.events:emit("custom:event", nil)            -- Handler sees {}
```

### Event Handler Context (Snapshots)

Event handlers receive a **snapshot** of compositor state at emission time:

```lua
niri.events:on("window:open", function(ev)
    -- niri.state.windows() returns a snapshot, not live data
    -- This prevents deadlocks and race conditions
    local windows = niri.state.windows()
end)
```

This is stored in a thread-local `EVENT_CONTEXT_STATE` and automatically cleared after the handler returns.

### Event Payload Design

Events provide **minimal, consistent payloads** - just enough to identify the subject. This design choice reflects:

1. **Fork maintenance**: Regular upstream merges. Rich payloads would require constructing full IPC objects at every emit site, increasing merge conflicts.
2. **Performance**: Avoids gathering all fields when handlers don't need them.
3. **Consistency**: All events of the same type have identical payload structure.

Handlers can query `niri.state.windows()` / `niri.state.workspaces()` for full details (see LUA_GUIDE.md).

### Available Events

#### Window Events
| Event | Payload | When |
|-------|---------|------|
| `window:open` | `{id, title}` | Window created |
| `window:close` | `{id, title}` | Window destroyed |
| `window:focus` | `{id, title}` | Window gained focus |
| `window:blur` | `{id, title}` | Window lost focus |
| `window:title_changed` | `{id, title}` | Title updated |
| `window:app_id_changed` | `{id, app_id}` | App ID updated |
| `window:fullscreen` | `{id, title, is_fullscreen}` | Fullscreen toggled |
| `window:maximize` | `{id, title, is_maximized}` | Maximize toggled |
| `window:resize` | `{id, title, width, height}` | Window resized |
| `window:move` | `{id, title, from_workspace?, to_workspace, from_output?, to_output}` | Window moved |

#### Workspace Events
| Event | Payload | When |
|-------|---------|------|
| `workspace:activate` | `{name, idx}` | Workspace became active |
| `workspace:deactivate` | `{name, idx}` | Workspace became inactive |
| `workspace:create` | `{name, idx, output}` | Workspace created |
| `workspace:destroy` | `{name, idx, output}` | Workspace destroyed |
| `workspace:rename` | `{idx, old_name?, new_name?, output}` | Workspace renamed |

#### Layout Events
| Event | Payload | When |
|-------|---------|------|
| `layout:mode_changed` | `{mode}` | Layout mode changed (mode: "floating"\|"tiling") |
| `layout:window_added` | `{id}` | Window added to layout |
| `layout:window_removed` | `{id}` | Window removed from layout |

#### System Events
| Event | Payload | When |
|-------|---------|------|
| `config:reload` | `{success}` | Configuration reloaded |
| `overview:open` | `{}` | Overview mode opened |
| `overview:close` | `{}` | Overview mode closed |

#### Monitor Events
| Event | Payload | When |
|-------|---------|------|
| `monitor:connect` | `{name, connector}` | Monitor connected |
| `monitor:disconnect` | `{name, connector}` | Monitor disconnected |

#### Output Events
| Event | Payload | When |
|-------|---------|------|
| `output:mode_change` | `{output, width, height, refresh_rate?}` | Output resolution/refresh changed |

#### Lock Events
| Event | Payload | When |
|-------|---------|------|
| `lock:activate` | `{}` | Screen locked |
| `lock:deactivate` | `{}` | Screen unlocked |

#### Lifecycle Events
| Event | Payload | When |
|-------|---------|------|
| `startup` | `{}` | Compositor finished initializing |
| `shutdown` | `{}` | Compositor shutting down |

### Excluded Events (Security)

The following events are intentionally NOT exposed:
- `key:press`, `key:release` - Security (keylogging), performance (every keystroke)
- `idle:start`, `idle:end` - Not exposed via Smithay's IdleNotifierState

---

## Action System

The `niri.action` namespace provides access to ~90 compositor actions.

### Architecture

- **Source file**: `action_proxy.rs` (1,160 LOC)
- **Pattern**: Method factories that return action objects
- **Macro**: `register_actions!` for no-argument actions

### Action Categories

#### Window Management
```lua
niri.action:close_window()
niri.action:fullscreen_window()
niri.action:maximize_column()
niri.action:toggle_window_floating()
niri.action:focus_window_up()
niri.action:focus_window_down()
niri.action:move_window_up()
niri.action:move_window_down()
niri.action:focus_window(id)              -- id: integer
```

#### Column Management
```lua
niri.action:focus_column_left()
niri.action:focus_column_right()
niri.action:move_column_left()
niri.action:move_column_right()
niri.action:center_column()
niri.action:toggle_column_tabbed()
niri.action:set_column_width({proportion = 0.5})
niri.action:set_column_width({fixed = 800})
```

#### Workspace Navigation
```lua
niri.action:focus_workspace(ref)          -- ref: integer | string
niri.action:focus_workspace_up()
niri.action:focus_workspace_down()
niri.action:move_window_to_workspace(ref)
niri.action:move_window_to_workspace_up()
niri.action:move_window_to_workspace_down()
```

#### Monitor Navigation
```lua
niri.action:focus_monitor_left()
niri.action:focus_monitor_right()
niri.action:focus_monitor_up()
niri.action:focus_monitor_down()
niri.action:move_window_to_monitor_left()
niri.action:move_window_to_monitor_right()
```

#### Process Spawning

The spawn API provides two modes: fire-and-forget for simple process launching, and managed mode with a ProcessHandle for controlling and monitoring processes.

**Basic usage (fire-and-forget):**
```lua
niri.action:spawn({"cmd", "arg1", "arg2"})  -- Array of strings, returns nil
niri.action:spawn_sh("shell command")        -- Shell string (Since 25.05), returns nil
```

**Advanced usage (with options and callbacks):**
```lua
-- Single-argument form (fire-and-forget)
niri.action:spawn({"kitty"})  -- Returns nil, spawns immediately

-- Two-argument form with options (returns ProcessHandle)
local handle = niri.action:spawn({"kitty"}, {
    stdout = function(err, data)
        if data then niri.utils.log("stdout: " .. data) end
    end,
    stderr = function(err, data)
        if data then niri.utils.log("stderr: " .. data) end
    end,
    on_exit = function(result)
        niri.utils.log("Process exited with code: " .. tostring(result.code))
    end
})

-- Capture output without streaming (wait for process to complete)
local handle = niri.action:spawn({"echo", "hello"}, {
    capture_stdout = true,  -- Capture stdout for wait() result
    capture_stderr = true,  -- Capture stderr for wait() result
})
local result = handle:wait()
print("stdout: " .. result.stdout)
print("stderr: " .. result.stderr)

-- Writing to stdin
local handle = niri.action:spawn({"cat"}, {
    stdin = true,  -- Enable stdin pipe (or stdin = "pipe")
    capture_stdout = true,
})
handle:write("Hello from Lua!\n")
handle:close_stdin()  -- Close stdin to signal EOF
local result = handle:wait()
print(result.stdout)  -- "Hello from Lua!"

-- Pass initial data to stdin (closes automatically after writing)
local handle = niri.action:spawn({"wc", "-l"}, {
    stdin = "line1\nline2\nline3\n",  -- Data string sent to stdin
    capture_stdout = true,
})
local result = handle:wait()
print(result.stdout)  -- "3"
```

**SpawnOpts fields:**

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `cwd` | `string?` | `nil` | Working directory for the process |
| `env` | `table<string, string>?` | `nil` | Environment variables to set (merged with parent) |
| `clear_env` | `boolean` | `false` | If `true`, start with empty environment (only use `env` table) |
| `stdin` | `boolean \| string` | `false` | `false`=closed, `true`/`"pipe"`=pipe for writing, `"data"`=send string then close |
| `capture_stdout` | `boolean` | `false` | Capture stdout for `wait()` result |
| `capture_stderr` | `boolean` | `false` | Capture stderr for `wait()` result |
| `stdout` | `boolean \| function` | `nil` | `true` to capture, or callback `function(err, data)` for streaming |
| `stderr` | `boolean \| function` | `nil` | `true` to capture, or callback `function(err, data)` for streaming |
| `on_exit` | `function(result)` | `nil` | Called when process exits with `{code, signal, stdout?, stderr?}` |
| `text` | `boolean` | `true` | If `true`, data is UTF-8 string; if `false`, data is raw bytes |
| `detach` | `boolean` | `false` | If `true`, returns `nil` (fire-and-forget mode) |

**Stdin modes:**

| Value | Behavior |
|-------|----------|
| `false` or `nil` | Stdin is closed (default) |
| `true` or `"pipe"` | Stdin is a pipe; use `handle:write()` and `handle:close_stdin()` |
| `"any other string"` | String is written to stdin, then stdin closes automatically |

**ProcessHandle methods:**

| Method | Description |
|--------|-------------|
| `handle:wait(timeout_ms?)` | Block until exit; returns result table. With timeout, sends SIGTERM then SIGKILL after 1s grace period |
| `handle:kill(signal?)` | Send signal to process. Default: SIGTERM. Accepts integer or string ("TERM", "KILL", "INT", "HUP", "QUIT", "USR1", "USR2") |
| `handle:write(data)` | Write data to stdin pipe. Requires `stdin = true` or `stdin = "pipe"` |
| `handle:close_stdin()` | Close the stdin pipe (signals EOF to the process) |
| `handle:is_closing()` | Returns `true` if stdin has been closed |
| `handle.pid` | Process ID (read-only integer) |

**Result table (from `wait()` or `on_exit` callback):**

```lua
{
    code = 0,           -- Exit code (integer, or nil if killed by signal)
    signal = nil,       -- Signal number (integer, or nil if normal exit)
    stdout = "output",  -- Captured stdout (if capture_stdout=true or stdout=true)
    stderr = "",        -- Captured stderr (if capture_stderr=true or stderr=true)
}
```

**Streaming callback signature:**

```lua
-- stdout/stderr callbacks receive (err, data)
function(err, data)
    if err then
        niri.utils.error("Stream error: " .. err)
    elseif data then
        -- data is a string (text mode) or bytes (binary mode)
        -- In text mode, each callback receives one line (without newline)
        niri.utils.log("Received: " .. data)
    end
end
```

**Timeout escalation behavior:**

When `wait(timeout_ms)` is called with a timeout:
1. Wait for process to exit within `timeout_ms` milliseconds
2. If timeout expires, send SIGTERM to the process
3. Wait up to 1000ms (grace period) for graceful termination
4. If process still running, send SIGKILL (cannot be ignored)
5. Return the result (signal will be 15 for SIGTERM or 9 for SIGKILL)

**Important notes:**

- **GC behavior**: ProcessHandle garbage collection does NOT kill the OS process (matches Neovim behavior). Processes continue running independently.
- **Callback invocation**: All callbacks (stdout, stderr, on_exit) are invoked on the main compositor thread via an event queue.
- **Return value logic**: `spawn()` returns `nil` if called without opts, with `detach = true`, or if spawn fails. Otherwise returns ProcessHandle.

**Spawning at Startup:**

> ⚠️ **WARNING**: When `niri.action:spawn()` is called during config load (synchronously), spawned processes may not work correctly. To spawn processes at startup, use one of these approaches instead:

```lua
-- Option 1: Use startup event
niri.events:on("startup", function()
    niri.action:spawn({"waybar"})
end)

-- Option 2: Use niri.schedule()
niri.schedule(function()
    niri.action:spawn({"mako"})
end)

-- Option 3: Use timer with delay
niri.loop.new_timer():start(100, 0, function()
    niri.action:spawn({"swaybg", "-i", "wallpaper.png"})
end)
```

#### Screenshots
```lua
niri.action:screenshot()
niri.action:screenshot_window()
niri.action:screenshot_screen()
```

#### System
```lua
niri.action:quit()
niri.action:power_off_monitors()
niri.action:suspend()
niri.action:toggle_overview()
niri.action:show_hotkey_overlay()
```

### Parameterized Actions

Actions with parameters use table arguments:

```lua
-- Size changes
niri.action:set_column_width({proportion = 0.5})  -- 50% of screen
niri.action:set_column_width({fixed = 800})       -- 800 pixels
niri.action:set_window_height({proportion = 0.333})

-- Workspace reference (string name or integer index)
niri.action:focus_workspace("browser")
niri.action:focus_workspace(3)

-- Layout switch
niri.action:switch_layout("next")
niri.action:switch_layout("prev")
niri.action:switch_layout(0)  -- Specific index
```

### Using Actions in Bindings

Actions are called as factories that return action objects:

```lua
-- CORRECT: Call the action factory
niri.config.binds:add("Mod+Q", niri.action:close_window())

-- WRONG: Missing parentheses
niri.config.binds:add("Mod+Q", niri.action:close_window)  -- Won't work!
```

---

## Timer API

The `niri.loop` namespace provides Neovim-style timer functionality.

### Architecture

- **Source file**: `loop_api.rs` (200+ LOC)
- **Pattern**: `TimerManager` with `HashMap` + `BinaryHeap<Reverse<(Instant, u64)>>`
- **Lifetime**: Timers persist until explicit `close()` (GC doesn't stop them)

### Timer Creation

```lua
-- One-shot timer (fires once after delay)
local timer = niri.loop.new_timer()
timer:start(1000, 0, function()
    print("Fired after 1 second!")
end)

-- Repeating timer (fires every interval)
local timer = niri.loop.new_timer()
timer:start(1000, 1000, function()
    print("Fires every 1 second!")
end)
```

### Timer Methods

| Method | Description |
|--------|-------------|
| `timer:start(delay_ms, repeat_ms, callback)` | Start timer with delay and optional repeat |
| `timer:stop()` | Stop timer (can be restarted with `again()`) |
| `timer:again()` | Restart with same settings |
| `timer:close()` | Permanently destroy timer |
| `timer:is_active()` | Check if timer is running |
| `timer:get_due_in()` | Returns ms until next fire (0 when inactive) |
| `timer:set_repeat(ms)` | Change repeat interval at runtime |
| `timer:get_repeat()` | Returns current repeat interval (ms) |
| `timer.id` | Read-only numeric timer ID |

### Current Time

```lua
local now = niri.loop.now()  -- Monotonic time in milliseconds
```

### wait() Function

The `wait()` function provides convenient sleep and condition-polling:

```lua
-- Signature: niri.loop.wait(timeout_ms, condition_fn?, interval_ms?) -> (ok, value)
```

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `timeout_ms` | integer | required | Maximum time to wait |
| `condition_fn` | function | nil | Optional function to poll |
| `interval_ms` | integer | 10 | Polling interval (minimum 1ms) |

**Return values:**
- `(true, nil)` - Timeout elapsed (when no condition provided)
- `(true, value)` - Condition returned truthy value
- `(false, nil)` - Timeout elapsed before condition was satisfied

**Behavior:**
- Without `condition_fn`: Sleeps for `timeout_ms` and returns `(true, nil)`
- With `condition_fn`: Polls every `interval_ms`
  - Returns immediately when condition returns truthy (anything except `nil` or `false`)
  - The returned `value` is whatever the condition function returned
  - Interval is clamped to minimum 1ms to prevent busy-spin
  - Errors in condition function propagate to caller

**Examples:**
```lua
-- Simple sleep
niri.loop.wait(1000)  -- Sleep 1 second

-- Wait for window to appear
local ok, window = niri.loop.wait(5000, function()
    local windows = niri.state.windows()
    for _, w in ipairs(windows) do
        if w.app_id == "firefox" then return w end
    end
    return false
end, 50)

-- Truthiness: 0, "", {} are truthy in Lua
local ok, val = niri.loop.wait(100, function() return 0 end)
-- ok=true, val=0 (0 is truthy)
```

### Timer Lifetime

**Important**: Timers continue running even if the Lua handle is garbage collected:

```lua
-- This timer will still fire even though we don't keep the handle!
niri.loop.new_timer():start(1000, 0, function()
    print("Still fires!")
end)

-- Proper cleanup pattern
local timer = niri.loop.new_timer()
timer:start(1000, 0, function()
    timer:close()  -- Explicit cleanup after firing
end)
```

This matches Neovim's `vim.uv` semantics for familiarity.

---

## API Patterns

> **For LLM Agents**: Use this decision tree to determine which API pattern to use.

### Pattern Decision Tree

```
WHEN accessing niri.config.*:
│
├─ IS the field a COLLECTION? (binds, outputs, window_rules, layer_rules, workspaces, environment)
│   └─ YES → Use collection methods: :add(), :remove(), :list(), :get(), :clear(), :set()
│       │
│       ├─ Adding items:
│       │   binds:add(key, action)           -- Two-arg form for bindings
│       │   binds:add(key, {action=..., ...}) -- Table form with options
│       │   outputs:add({name="...", ...})   -- Table form for outputs
│       │   window_rules:add({matches=...})  -- Table form for rules
│       │   environment:add(key, value)      -- Key-value form
│       │
│       ├─ Querying:
│       │   :list()  → Returns array of all items
│       │   :get(key) → Returns single item or nil
│       │
│       └─ Modifying:
│           :remove(key) → Remove by key
│           :clear()     → Remove all
│           :set(table)  → Replace entire collection
│
├─ IS the field a NESTED TABLE? (input, layout, cursor, animations, gestures, etc.)
│   └─ YES → Use direct assignment or nested access
│       │
│       ├─ Full replacement:
│       │   niri.config.input = { keyboard = {...}, touchpad = {...} }
│       │
│       └─ Nested access:
│           niri.config.input.keyboard.xkb.layout = "us"
│           niri.config.layout.gaps = 16
│           niri.config.layout.focus_ring.width = 4
│
└─ IS the field a SCALAR? (prefer_no_csd, screenshot_path)
    └─ YES → Use direct assignment
        niri.config.prefer_no_csd = true
        niri.config.screenshot_path = "~/Pictures/%Y-%m-%d.png"


WHEN using niri.action:*:
│
├─ Action takes NO arguments:
│   niri.action:close_window()
│   niri.action:quit()
│
├─ Action takes a SINGLE primitive:
│   niri.action:focus_window(id)           -- integer
│   niri.action:focus_workspace(ref)       -- integer | string
│
├─ Action takes an ARRAY:
│   niri.action:spawn({"cmd", "arg1"})     -- string[]
│
└─ Action takes a TABLE:
    niri.action:set_column_width({proportion = 0.5})
    niri.action:set_window_height({fixed = 600})


WHEN using niri.events.*:
│
├─ Subscribe (single):    niri.events:on("event:name", function(ev) ... end)
├─ Subscribe (multiple):  niri.events:on({"event1", "event2"}, function(ev) ... end)
├─ Once (single):         niri.events:once("event:name", function(ev) ... end)
├─ Once (multiple):       niri.events:once({"event1", "event2"}, function(ev) ... end)
├─ Remove (single):       niri.events:off("event:name", handler_id)
├─ Remove (multiple):     niri.events:off(handler_ids_table)  -- table from :on()/:once()
└─ List:                  niri.events:list() → {"event:name", ...}


WHEN using niri.state.*:
│
└─ Read-only access:
    local windows = niri.state.windows()         -- array
    local focused = niri.state.focused_window()  -- table | nil
```

### Common Mistakes (Avoid These)

```lua
-- WRONG: Using assignment for collections
niri.config.binds["Mod+Q"] = action  -- ❌ Won't work

-- RIGHT: Use :add() method
niri.config.binds:add("Mod+Q", action)  -- ✓

-- WRONG: Using :add() for tables
niri.config.layout:add({gaps = 16})  -- ❌ layout is not a collection

-- RIGHT: Use direct assignment
niri.config.layout.gaps = 16  -- ✓

-- WRONG: Calling action without parentheses
niri.config.binds:add("Mod+Q", niri.action:close_window)  -- ❌ Missing ()

-- RIGHT: Call the action factory
niri.config.binds:add("Mod+Q", niri.action:close_window())  -- ✓

-- WRONG: spawn with string
niri.action:spawn("alacritty")  -- ❌ Expects array

-- RIGHT: spawn with array
niri.action:spawn({"alacritty"})  -- ✓

-- RIGHT: spawn_sh with string
niri.action:spawn_sh("alacritty --working-directory ~")  -- ✓
```

---

## Schema Reference

> **For LLM Agents**: Machine-readable type definitions. Use these for validation and code generation.

### Primitive Types

```yaml
types:
  integer:
    lua_type: number
    constraint: "math.floor(x) == x"

  float:
    lua_type: number

  boolean:
    lua_type: boolean
    values: [true, false]

  string:
    lua_type: string

  color:
    lua_type: string
    pattern: "^#([0-9a-fA-F]{3}|[0-9a-fA-F]{4}|[0-9a-fA-F]{6}|[0-9a-fA-F]{8})$"
    examples: ["#fff", "#ffffff", "#ffffff80"]
    invalid: ["red", "rgb(255,0,0)", "#gg0000"]

  proportion:
    lua_type: number
    range: [0.0, 1.0]
    examples: [0.5, 0.333, 1.0]

  angle:
    lua_type: number
    range: [0, 360]
    unit: degrees
    wrap: true  # 370 -> 10
```

### Enum Types

```yaml
enums:
  center_focused_column:
    values: ["never", "always", "on-overflow"]
    default: "never"

  accel_profile:
    values: ["adaptive", "flat"]
    default: "adaptive"

  click_method:
    values: ["button-areas", "clickfinger"]
    default: "button-areas"

  scroll_method:
    values: ["two-finger", "edge", "on-button-down"]
    default: "two-finger"

  track_layout:
    values: ["global", "window"]
    default: "global"

  transform:
    values: ["normal", "90", "180", "270", "flipped", "flipped-90", "flipped-180", "flipped-270"]
    default: "normal"

  variable_refresh_rate:
    values: [true, false, "on-demand", "hint"]
    default: false

  relative_to:
    values: ["workspace-view", "window"]
    default: "workspace-view"

  block_out_from:
    values: ["screencast", "screen-capture"]

  easing_curve:
    values: ["linear", "ease-out-quad", "ease-out-cubic", "ease-out-expo", "ease-out-quint"]
    custom: "array of 4 floats for cubic bezier: [x1, y1, x2, y2]"

  focus_follows_mouse:
    values: [true, false, "max-scroll-amount"]
    default: false

  column_display:
    values: ["normal", "tabbed"]
    default: "normal"
    since: "25.11"

  tab_indicator_position:
    values: ["left", "right", "top", "bottom"]
    default: "left"
    since: "25.05"
  
  focus_mode:
    values: ["normal", "overview", "layer_shell", "locked"]
    default: "normal"
    since: "25.XX"
    description: "Current compositor focus mode"
```

### State API Types (Since 25.XX)

```yaml
cursor_position:
  type: object
  nullable: true
  properties:
    x: { type: float, description: "X coordinate in global space (pixels)" }
    y: { type: float, description: "Y coordinate in global space (pixels)" }
    output: { type: string, description: "Name of output where cursor is located" }
  returns_nil_when:
    - "No pointing device connected"
    - "Cursor grabbed by surface (resize, move)"
    - "Cursor position undefined"
  example:
    x: 1920.5
    y: 540.0
    output: "eDP-1"

reserved_space:
  type: object
  properties:
    top: { type: integer, min: 0, description: "Pixels reserved at top edge" }
    bottom: { type: integer, min: 0, description: "Pixels reserved at bottom edge" }
    left: { type: integer, min: 0, description: "Pixels reserved at left edge" }
    right: { type: integer, min: 0, description: "Pixels reserved at right edge" }
  note: "Returns maximum exclusive zone per edge, not sum"
  example:
    top: 32
    bottom: 0
    left: 0
    right: 0
```

### Size Types

```yaml
size_spec:
  oneOf:
    - type: object
      properties:
        proportion: { type: float, range: [0.0, 1.0] }
    - type: object
      properties:
        fixed: { type: integer, min: 1 }
  examples:
    - { proportion: 0.5 }
    - { fixed: 800 }
```

### Gradient Type

```yaml
gradient:
  type: object
  properties:
    from: { type: color, required: true }
    to: { type: color, required: true }
    angle: { type: angle, default: 0 }
    relative_to: { type: relative_to, default: "workspace-view" }
    in_: { type: string, default: "srgb", since: "25.05" }
  example:
    from: "#80c8ff"
    to: "#bbddff"
    angle: 45
    relative_to: "workspace-view"
```

### Configuration Section Schemas

```yaml
input:
  type: object
  properties:
    keyboard:
      type: object
      properties:
        xkb:
          type: object
          properties:
            layout: { type: string, default: "" }
            variant: { type: string, default: "" }
            options: { type: string, default: "" }
            model: { type: string, default: "" }
            rules: { type: string, default: "" }
        repeat_delay: { type: integer, range: [1, 10000], default: 600 }
        repeat_rate: { type: integer, range: [1, 1000], default: 25 }
        track_layout: { type: track_layout, default: "global" }
        numlock: { type: boolean, default: false, since: "25.05" }

    touchpad:
      type: object
      properties:
        tap: { type: boolean, default: true }
        dwt: { type: boolean, default: true }
        dwtp: { type: boolean, default: true }
        natural_scroll: { type: boolean, default: false }
        accel_speed: { type: float, range: [-1.0, 1.0], default: 0.0 }
        accel_profile: { type: accel_profile, default: "adaptive" }
        tap_button_map: { type: string, default: "left-right-middle" }
        click_method: { type: click_method, default: "button-areas" }
        scroll_method: { type: scroll_method, default: "two-finger" }
        disabled: { type: boolean, default: false }

    mouse:
      type: object
      properties:
        natural_scroll: { type: boolean, default: false }
        accel_speed: { type: float, range: [-1.0, 1.0], default: 0.0 }
        accel_profile: { type: accel_profile, default: "flat" }
        scroll_button: { type: integer, default: 274 }
        scroll_button_lock: { type: boolean, default: false }
        disabled: { type: boolean, default: false }

    trackpoint:
      type: object
      since: "25.05"
      properties:
        natural_scroll: { type: boolean, default: false }
        accel_speed: { type: float, range: [-1.0, 1.0], default: 0.0 }
        accel_profile: { type: accel_profile, default: "flat" }
        scroll_method: { type: scroll_method, default: "on-button-down" }
        scroll_button: { type: integer, default: 274 }
        disabled: { type: boolean, default: false }

    tablet:
      type: object
      properties:
        map_to_output: { type: string, nullable: true }

    touch:
      type: object
      properties:
        map_to_output: { type: string, nullable: true }

    mod_key: { type: string, default: "Super", since: "25.05" }
    mod_key_nested: { type: string, default: "Super", since: "25.05" }
    power_key_handling: { type: string, enum: ["suspend", "ignore"], default: "suspend" }
    disable_power_key_handling: { type: boolean, default: false }
    warp_mouse_to_focus: { type: boolean, default: false }
    focus_follows_mouse: { type: focus_follows_mouse, default: false }
    workspace_auto_back_and_forth: { type: boolean, default: false }

layout:
  type: object
  properties:
    gaps: { type: integer, range: [0, 1000], default: 16 }
    center_focused_column: { type: center_focused_column, default: "never" }
    always_center_single_column: { type: boolean, default: false, since: "25.05" }
    background_color: { type: color, nullable: true, since: "25.05" }
    default_column_display: { type: column_display, default: "normal", since: "25.11" }
    empty_workspace_above_first: { type: boolean, default: false, since: "25.05" }
    struts:
      type: object
      properties:
        left: { type: integer, default: 0 }
        right: { type: integer, default: 0 }
        top: { type: integer, default: 0 }
        bottom: { type: integer, default: 0 }
    preset_column_widths:
      type: array
      items: { type: size_spec }
      default: [{ proportion: 0.333 }, { proportion: 0.5 }, { proportion: 0.666 }]
    default_column_width: { type: size_spec, default: { proportion: 0.5 } }
    preset_window_heights:
      type: array
      items: { type: size_spec }
      default: [{ proportion: 0.333 }, { proportion: 0.5 }, { proportion: 0.666 }]
    focus_ring:
      type: object
      properties:
        off: { type: boolean, default: false }
        width: { type: integer, range: [0, 100], default: 4 }
        active_color: { type: color, default: "#7fc8ff" }
        inactive_color: { type: color, default: "#505050" }
        active_gradient: { type: gradient, nullable: true }
        inactive_gradient: { type: gradient, nullable: true }
    border:
      type: object
      properties:
        off: { type: boolean, default: true }
        width: { type: integer, range: [0, 100], default: 4 }
        active_color: { type: color, default: "#ffc87f" }
        inactive_color: { type: color, default: "#505050" }
        active_gradient: { type: gradient, nullable: true }
        inactive_gradient: { type: gradient, nullable: true }
    shadow:
      type: object
      properties:
        on: { type: boolean, default: false }
        softness: { type: integer, range: [0, 100], default: 30 }
        spread: { type: integer, range: [0, 100], default: 5 }
        offset: { type: object, properties: { x: integer, y: integer }, default: { x: 0, y: 5 } }
        color: { type: color, default: "#00000070" }
        inactive_color: { type: color, default: "#00000040" }
        corner_radius: { type: float, range: [0, 100], default: 12.0, since: "25.05" }
    tab_indicator:
      type: object
      properties:
        off: { type: boolean, default: false }
        hide_when_single_tab: { type: boolean, default: true }
        place_within_column: { type: boolean, default: false, since: "25.05" }
        gap: { type: float, default: 10.0 }
        width: { type: float, default: 4.0 }
        length: { type: size_spec, default: { proportion: 0.3 } }
        corner_radius: { type: float, default: 8.0 }
        active_color: { type: color, default: "#ffc87f" }
        inactive_color: { type: color, default: "#505050" }
        position: { type: tab_indicator_position, default: "left", since: "25.05" }
    insert_hint:
      type: object
      properties:
        off: { type: boolean, default: false }
        color: { type: color, default: "#ffc87f80" }
        gradient: { type: gradient, nullable: true }
```

### Window Rule Schema

```yaml
window_rule:
  type: object
  properties:
    matches:
      type: array
      items:
        type: object
        properties:
          app_id: { type: string, nullable: true }
          title: { type: string, nullable: true }
          is_regex: { type: boolean, default: false }
          is_focused: { type: boolean, nullable: true }
          is_active: { type: boolean, nullable: true }
          is_active_in_column: { type: boolean, nullable: true, since: "25.05" }
          is_floating: { type: boolean, nullable: true }
          is_window_cast_target: { type: boolean, nullable: true, since: "25.11" }
          is_urgent: { type: boolean, nullable: true, since: "25.11" }
          at_startup: { type: boolean, nullable: true }
    excludes:
      type: array
      items: { type: match_criteria }
    # Properties
    open_on_output: { type: string, nullable: true }
    open_on_workspace: { type: string, nullable: true }
    open_maximized: { type: boolean, nullable: true }
    open_fullscreen: { type: boolean, nullable: true }
    open_floating: { type: boolean, nullable: true }
    default_column_width: { type: size_spec, nullable: true }
    default_window_height: { type: size_spec, nullable: true }
    default_floating_position:
      type: object
      nullable: true
      properties:
        x: { type: integer }
        y: { type: integer }
        relative_to: { type: string, enum: ["top-left", "top-right", "bottom-left", "bottom-right", "center"] }
    opacity: { type: float, range: [0.0, 1.0], nullable: true }
    draw_border_with_background: { type: boolean, nullable: true }
    geometry_corner_radius: { type: float, nullable: true, since: "25.05" }
    clip_to_geometry: { type: boolean, nullable: true, since: "25.05" }
    focus_ring: { type: focus_ring, nullable: true, since: "25.11" }
    border: { type: border, nullable: true, since: "25.11" }
    shadow: { type: shadow, nullable: true, since: "25.11" }
    tab_indicator: { type: tab_indicator, nullable: true, since: "25.11" }
    block_out_from: { type: block_out_from, nullable: true }
    scroll_factor: { type: float, nullable: true, since: "25.11" }
    baba_is_float: { type: boolean, nullable: true, since: "25.11" }
    tiled_state: { type: boolean, nullable: true, since: "25.11" }
    min_width: { type: integer, nullable: true }
    max_width: { type: integer, nullable: true }
    min_height: { type: integer, nullable: true }
    max_height: { type: integer, nullable: true }
    default_column_display: { type: column_display, nullable: true, since: "25.11" }
```

### Output Schema

```yaml
output:
  type: object
  properties:
    name: { type: string, required: true, description: "Connector name, e.g., eDP-1, HDMI-A-1" }
    off: { type: boolean, default: false }
    mode:
      type: object
      nullable: true
      properties:
        width: { type: integer, required: true }
        height: { type: integer, required: true }
        refresh: { type: float, nullable: true }
    modeline: { type: string, nullable: true, since: "25.05" }
    scale: { type: float, range: [0.25, 10.0], default: 1.0 }
    transform: { type: transform, default: "normal" }
    position:
      type: object
      nullable: true
      properties:
        x: { type: integer }
        y: { type: integer }
    variable_refresh_rate: { type: variable_refresh_rate, default: false }
    background_color: { type: color, nullable: true }
    backdrop_color: { type: color, nullable: true, since: "25.05" }
    focus_at_startup: { type: boolean, default: false }
    # Per-output layout overrides (Since 25.11)
    default_column_width: { type: size_spec, nullable: true }
    focus_ring: { type: focus_ring, nullable: true }
```

### Animation Schema

```yaml
animation_config:
  type: object
  oneOf:
    - spring:
        type: object
        properties:
          damping_ratio: { type: float, range: [0.0, 10.0], default: 1.0 }
          stiffness: { type: float, range: [0.0, 10000.0], default: 1000 }
          epsilon: { type: float, range: [0.0, 1.0], default: 0.0001 }
    - easing:
        type: object
        properties:
          duration_ms: { type: integer, range: [0, 10000] }
          curve: { type: easing_curve }

animations:
  type: object
  properties:
    off: { type: boolean, default: false }
    slowdown: { type: float, range: [0.0, 100.0], default: 1.0 }
    workspace_switch: { type: animation_config }
    window_open: { type: animation_config }
    window_close: { type: animation_config }
    window_movement: { type: animation_config }
    window_resize: { type: animation_config }
    horizontal_view_movement: { type: animation_config }
    config_notification_open_close: { type: animation_config }
    screenshot_ui_open: { type: animation_config }
    overview_open_close: { type: animation_config }
    exit_confirmation_open_close: { type: animation_config }
    recent_windows_close: { type: animation_config, since: "25.11" }
    shaders:
      type: object
      nullable: true
      since: "25.05"
      properties:
        window_open: { type: string, nullable: true }
        window_close: { type: string, nullable: true }
```

### Binding Schema

```yaml
binding:
  key_pattern:
    type: string
    pattern: "^(Mod\\+|Ctrl\\+|Shift\\+|Alt\\+|Super\\+|ISO_Level3_Shift\\+)*(Mouse(Left|Right|Middle|Forward|Back)|Wheel(ScrollUp|ScrollDown)|Touchpad(ScrollUp|ScrollDown)|[A-Za-z0-9_]+)$"
    examples:
      - "Mod+Q"
      - "Mod+Shift+Return"
      - "Mod+MouseLeft"
      - "Mod+WheelScrollDown"
      - "XF86AudioRaiseVolume"

  binding_options:
    type: object
    properties:
      action: { type: action, required: true }
      allow_when_locked: { type: boolean, default: false }
      allow_inhibiting: { type: boolean, default: true }
      cooldown_ms: { type: integer, range: [0, 10000], nullable: true }
      repeat_: { type: boolean, default: false, since: "25.05" }
      hotkey_overlay_title: { type: string, nullable: true }
```

---

The `niri.config` namespace provides full KDL configuration parity. Configurations are specified as Lua tables and converted to niri's internal `Config` struct.

### Basic Usage

```lua
local niri = require("niri")

niri.config.input.keyboard.xkb.layout = "us"
niri.config.input.touchpad.tap = true

niri.config.layout.gaps = 16
niri.config.layout.center_focused_column = "never"

-- Bindings use collection proxies
niri.config.binds:add("Mod+Return", niri.action:spawn({"alacritty"}))
niri.config.binds:add("Mod+Q", niri.action:close_window())
```

### Configuration Fields

| Field | Type | Description |
|-------|------|-------------|
| `input` | table | Input device configuration (keyboard, touchpad, mouse, tablet, touch, trackpoint) |
| `outputs` | collection | Monitor configuration (mode, scale, transform, position, vrr) |
| `layout` | table | Tiling layout settings (gaps, focus_ring, border, shadow, etc.) |
| `spawn_at_startup` | array | Commands to run at startup |
| `prefer_no_csd` | boolean | Prefer server-side decorations |
| `screenshot_path` | string | Screenshot save location (supports `%Y`, `%m`, `%d`, etc.) |
| `hotkey_overlay` | table | Hotkey overlay settings (skip_at_startup) |
| `environment` | collection | Environment variables for spawned processes |
| `cursor` | table | Cursor theme, size, and hide settings |
| `binds` | collection | Key/mouse/scroll bindings |
| `window_rules` | collection | Window matching rules and properties |
| `layer_rules` | collection | Layer surface rules |
| `debug` | table | Debug options (preview_render, dbus_interfaces_in_non_session, etc.) |
| `animations` | table | Animation settings (11 animation types with spring/easing) |
| `gestures` | table | Touchpad gestures configuration |
| `overview` | table | Overview mode settings (backdrop_color) |
| `switch_events` | table | Lid and tablet switch event handling |
| `workspaces` | collection | Named workspace definitions |
| `recent_windows` | table | Recent windows UI settings (Since 25.11) |
| `clipboard` | table | Clipboard settings (disable_primary) |
| `xwayland_satellite` | table | XWayland-satellite settings (off, path) |
| `config_notification` | table | Config notification settings (disable_failed) |

---

## Module System

The module system enables users to organize Lua code into reusable modules relative to their config file.

### Config-Relative Require

Niri extends `package.path` to include the config file's directory, allowing `require()` to resolve modules relative to the config file. This follows the Neovim model for user modules.

**Search Order:**

1. Config file directory (e.g., `~/.config/niri/`)
2. Standard Lua paths (`package.path`)

**Supported Patterns:**

| Pattern | Resolves To |
|---------|-------------|
| `require("mymodule")` | `<config_dir>/mymodule.lua` |
| `require("mymodule")` | `<config_dir>/mymodule/init.lua` |
| `require("utils.helpers")` | `<config_dir>/utils/helpers.lua` |
| `require("utils.helpers")` | `<config_dir>/utils/helpers/init.lua` |

### Usage Example

```lua
-- User creates: ~/.config/niri/mymodule.lua
local M = {}
function M.setup(opts)
    -- Custom keybindings, window rules, etc.
    niri.config.layout.gaps = opts.gaps or 16
end
return M

-- In ~/.config/niri/config.lua:
local mymodule = require("mymodule")
mymodule.setup({ gaps = 20 })
```

### Nested Modules

```lua
-- ~/.config/niri/utils/keybinds.lua
local M = {}
function M.add_workspace_binds()
    for i = 1, 9 do
        niri.config.binds:add("Mod+" .. i, niri.action:focus_workspace(i))
    end
end
return M

-- In config.lua:
local keybinds = require("utils.keybinds")
keybinds.add_workspace_binds()
```

### init.lua Convention

Directories can contain an `init.lua` file that is loaded when requiring the directory name:

```
~/.config/niri/
├── config.lua
└── themes/
    └── init.lua    -- require("themes") loads this
```

### Design Decisions

Following the Neovim model:

1. **Config-relative first**: Modules are resolved relative to the config file, making configurations portable
2. **Standard Lua semantics**: `require("foo.bar")` works exactly as expected
3. **No plugin manager**: Users load plugins explicitly via `require()` in their config
4. **No lifecycle management**: No `disable()`, no hot-reload (keep it simple)

---

## REPL

The REPL (Read-Eval-Print Loop) enables interactive Lua execution via IPC for debugging, testing, and exploration.

### Architecture

- **Source file**: `ipc_repl.rs` (78 LOC)
- **Entry point**: `niri msg lua "<code>"`
- **Pattern**: IPC command execution with output capture

### Core Components

```rust
// From ipc_repl.rs
pub struct IpcLuaExecutor {
    runtime: Arc<Mutex<Option<LuaRuntime>>>,
}

impl IpcLuaExecutor {
    pub fn execute(&self, code: &str) -> (String, bool) {
        // Returns (output, success)
    }
}
```

### Usage

```bash
# Execute Lua code
niri msg lua "print('Hello from REPL!')"

# Query state
niri msg lua "return niri.state.focused_window()"

# Multiple statements
niri msg lua "local w = niri.state.focused_window(); print(w.app_id)"

# Execute action
niri msg lua "niri.action:close_window()"
```

### Return Values

The REPL captures and displays return values:

```bash
$ niri msg lua "return 1 + 2"
3

$ niri msg lua "return niri.state.windows()"
{
  { id = 1, app_id = "firefox", title = "Mozilla Firefox" },
  { id = 2, app_id = "alacritty", title = "Terminal" },
}

$ niri msg lua "return niri.state.focused_window()"
{ id = 1, app_id = "firefox", title = "Mozilla Firefox" }
```

### Error Handling

Errors are returned with `success = false`:

```bash
$ niri msg lua "invalid syntax here"
[string "..."]:1: syntax error near 'syntax'

$ niri msg lua "error('custom error')"
[string "..."]:1: custom error
```

### Use Cases

1. **Debugging**: Inspect runtime state without reloading config
2. **Testing**: Verify API behavior interactively
3. **Scripting**: Execute one-off commands from shell scripts
4. **Exploration**: Discover available APIs and their return values

---

## Test Cleanup History

This section documents recent testing infrastructure cleanups and optimizations to maintain codebase health.

### December 2024: Integration Test Consolidation

#### Duplicate Test Removal (13 tests)

**Scope**: Eliminated test redundancy between `integration_tests.rs` and `repl_integration.rs`.

**Before**: 52 tests in `integration_tests.rs`
**After**: 39 tests in `integration_tests.rs`
**Coverage Impact**: None (all removed tests had equivalents in repl_integration.rs)

**Removed Test Categories**:
- Configuration API tests (11 tests)
  - `test_config_table_assignment` - Duplicate of repl validation
  - `test_collection_add_remove` - Duplicate of repl verification
  - `test_event_handler_registration` - Equivalent coverage in repl_integration.rs
- Edge case tests (2 tests)
  - `test_nil_values_in_tables` - Covered by repl edge cases
  - `test_invalid_enum_values` - Covered by repl error handling

**Retained Edge Cases**:
- Window state transitions (complex multi-step flows)
- Workspace layout changes under load
- Event emission ordering during compositor shutdown
- Timer cleanup when runtime is dropped

#### Unused Test Utilities Removal (8 functions)

**File**: `tests/test_utils.rs`
**Before**: 478 lines
**After**: 273 lines (-205 LOC)

**Removed Utilities**:

1. **TestDataBuilder struct** (62 LOC)
   - Purpose: Builder pattern for test data construction
   - Why removed: Only called in deleted tests; modern tests use inline table literals
   - Example: `TestDataBuilder::new().with_app_id("firefox").build()` → `{ app_id = "firefox" }`

2. **Type helper functions** (3 functions, ~35 LOC total)
   - `lua_number(val: f64) -> LuaValue`
   - `lua_integer(val: i64) -> LuaValue`
   - `lua_bool(val: bool) -> LuaValue`
   - Rationale: These were thin wrappers around mlua's `Value::` constructors; tests now use direct construction

3. **Lua code loading functions** (4 functions, ~108 LOC total)
   - `load_lua_code(path: &str) -> String`
   - `load_lua_config_file(path: &str) -> Result<Config>`
   - `load_and_validate_config(path: &str) -> (Config, Vec<Error>)`
   - `validate_config_field(name: &str, value: &str) -> bool`
   - Rationale: These performed file I/O for tests; modern tests use inline strings or fixtures

**Impact**: No test coverage loss; all removed functionality is now covered by:
- Inline test setup code (more readable)
- Direct Lua value construction (clearer intent)
- `tests/common.rs` helper functions (shared setup)

#### Runtime Creation Consolidation

**New Module**: `tests/common.rs`

**Purpose**: Centralized Lua runtime factory for all test suites.

**Shared Function**:
```rust
pub fn create_runtime() -> LuaRuntime {
    let lua = Lua::new();
    LuaRuntime::new(ExecutionLimits::default())
        .expect("Failed to create runtime")
}
```

**Benefits**:
- Both `integration_tests.rs` and `repl_integration.rs` use identical runtime setup
- Single point of change for runtime configuration
- Easier to add test fixtures (mock state, pre-configured bindings)
- Eliminates copy-paste helper functions in each test file

**Usage**:
```rust
// Before: Each test file had its own setup function
mod tests {
    fn setup_runtime() -> LuaRuntime { /* duplicated code */ }
}

// After: Shared from common module
use crate::common::create_runtime;
```

#### Orphaned Snapshot Removal (7 files)

**Location**: `src/snapshots/`

**Removed Snapshots**:
1. `validators__test_validation_schemas.rs.snap` (removed with `validators.rs` deletion)
2. `plugin_system__load_module.rs.snap` (module not implemented)
3. `plugin_system__resolve_path.rs.snap` (module not implemented)
4. `plugin_system__validate_manifest.rs.snap` (module not implemented)
5. `plugin_system__version_check.rs.snap` (module not implemented)
6. `plugin_system__cache_state.rs.snap` (module not implemented)
7. `plugin_system__error_formatting.rs.snap` (module not implemented)

**Why Orphaned**: Snapshots corresponded to code that was either:
- Deleted as dead code (validators.rs)
- Never implemented (plugin_system tests in codebase without corresponding implementation)

**Verification**: Confirmed via `cargo test --package niri-lua 2>&1 | grep "snapshot not found"` - all orphaned snapshots caused test failures until deleted.

### Testing Stats After Cleanup

| Metric | Before | After | Change |
|--------|--------|-------|--------|
| Total test count | 583 | 570 | -13 (-2.2%) |
| Test code LOC | ~2,100 | ~1,700 | -400 (-19%) |
| Test utilities LOC | 478 | 273 | -205 (-43%) |
| Integration tests | 52 | 39 | -13 (-25%) |
| Snapshot files | 47 | 40 | -7 orphaned |
| Test execution time | ~8.2s | ~7.5s | -0.7s (-8.5%) |

### Cleanup Verification

**No Coverage Loss**:
- All removed tests had equivalent coverage in other test suites
- Snapshot tests still verify all API schemas
- Integration tests still cover end-to-end workflows
- Coverage report shows 68% LOC coverage (unchanged)

**Quality Improvements**:
- Test code is now easier to read (less boilerplate)
- Maintenance burden reduced (fewer duplicate tests to update)
- Test execution is slightly faster (~8% improvement)

**Future Work**:
- Plugin system implementation will need new snapshot tests
- Additional edge case tests may be added as new features are implemented
- Test utilities can remain minimal and focused on shared setup only

---

## Appendix A: Canonical Reference Config

> **For LLM Agents**: A complete, working configuration showing all features. Copy and adapt.

```lua
-- CANONICAL REFERENCE CONFIGURATION
-- This file demonstrates all niri-lua configuration options with sensible defaults.
-- Copy this as a starting point and remove/modify sections as needed.

-- ============================================================================
-- INPUT CONFIGURATION
-- ============================================================================

niri.config.input = {
    keyboard = {
        xkb = {
            layout = "us",
            variant = "",
            options = "",
            model = "",
            rules = "",
        },
        repeat_delay = 600,
        repeat_rate = 25,
        track_layout = "global",
        numlock = false,
    },
    touchpad = {
        tap = true,
        dwt = true,
        dwtp = true,
        natural_scroll = true,
        accel_speed = 0.0,
        accel_profile = "adaptive",
        tap_button_map = "left-right-middle",
        click_method = "button-areas",
        scroll_method = "two-finger",
        disabled = false,
    },
    mouse = {
        natural_scroll = false,
        accel_speed = 0.0,
        accel_profile = "flat",
        scroll_button = 274,
        scroll_button_lock = false,
        disabled = false,
    },
    trackpoint = {
        natural_scroll = false,
        accel_speed = 0.0,
        accel_profile = "flat",
        scroll_method = "on-button-down",
        scroll_button = 274,
        disabled = false,
    },
    tablet = {
        map_to_output = nil,
    },
    touch = {
        map_to_output = nil,
    },
    power_key_handling = "suspend",
    disable_power_key_handling = false,
    warp_mouse_to_focus = false,
    focus_follows_mouse = false,
    workspace_auto_back_and_forth = false,
}

-- ============================================================================
-- LAYOUT CONFIGURATION
-- ============================================================================

niri.config.layout = {
    gaps = 16,
    center_focused_column = "never",
    always_center_single_column = false,
    default_column_display = "normal",
    empty_workspace_above_first = false,
    struts = { left = 0, right = 0, top = 0, bottom = 0 },
    preset_column_widths = {
        { proportion = 0.333 },
        { proportion = 0.5 },
        { proportion = 0.666 },
    },
    default_column_width = { proportion = 0.5 },
    preset_window_heights = {
        { proportion = 0.333 },
        { proportion = 0.5 },
        { proportion = 0.666 },
    },
    focus_ring = {
        off = false,
        width = 4,
        active_color = "#7fc8ff",
        inactive_color = "#505050",
    },
    border = {
        off = true,
        width = 4,
        active_color = "#ffc87f",
        inactive_color = "#505050",
    },
    shadow = {
        on = false,
        softness = 30,
        spread = 5,
        offset = { x = 0, y = 5 },
        color = "#00000070",
        inactive_color = "#00000040",
        corner_radius = 12.0,
    },
    tab_indicator = {
        off = false,
        hide_when_single_tab = true,
        place_within_column = false,
        gap = 10.0,
        width = 4.0,
        length = { proportion = 0.3 },
        corner_radius = 8.0,
        active_color = "#ffc87f",
        inactive_color = "#505050",
        position = "left",
    },
    insert_hint = {
        off = false,
        color = "#ffc87f80",
    },
}

-- ============================================================================
-- CURSOR CONFIGURATION
-- ============================================================================

niri.config.cursor = {
    xcursor_theme = "default",
    xcursor_size = 24,
    hide_when_typing = false,
    hide_after_inactive_ms = 0,
}

-- ============================================================================
-- ANIMATIONS
-- ============================================================================

niri.config.animations = {
    off = false,
    slowdown = 1.0,
    workspace_switch = {
        spring = { damping_ratio = 1.0, stiffness = 1000, epsilon = 0.0001 },
    },
    window_open = {
        easing = { duration_ms = 150, curve = "ease-out-expo" },
    },
    window_close = {
        easing = { duration_ms = 100, curve = "ease-out-quad" },
    },
    window_movement = {
        spring = { damping_ratio = 1.0, stiffness = 800, epsilon = 0.0001 },
    },
    window_resize = {
        spring = { damping_ratio = 1.0, stiffness = 800, epsilon = 0.0001 },
    },
    horizontal_view_movement = {
        spring = { damping_ratio = 1.0, stiffness = 800, epsilon = 0.0001 },
    },
    config_notification_open_close = {
        spring = { damping_ratio = 0.6, stiffness = 1000, epsilon = 0.001 },
    },
    screenshot_ui_open = {
        easing = { duration_ms = 200, curve = "ease-out-quad" },
    },
    overview_open_close = {
        easing = { duration_ms = 200, curve = "ease-out-expo" },
    },
}

-- ============================================================================
-- UI SETTINGS
-- ============================================================================

niri.config.hotkey_overlay = {
    skip_at_startup = false,
}

niri.config.overview = {
    backdrop_color = "#00000080",
}

niri.config.gestures = {
    workspace_swipe = {
        three_finger = true,
        four_finger = false,
        horizontal = true,
        distance = 400,
        natural_scroll = true,
    },
}

-- ============================================================================
-- MISCELLANEOUS
-- ============================================================================

niri.config.prefer_no_csd = true
niri.config.screenshot_path = "~/Pictures/Screenshots/%Y-%m-%d_%H-%M-%S.png"

niri.config.clipboard = {
    disable_primary = false,
}

niri.config.config_notification = {
    disable_failed = false,
}

-- ============================================================================
-- OUTPUTS
-- ============================================================================

-- Configure outputs (uncomment and modify as needed)
-- niri.config.outputs:add({
--     name = "eDP-1",
--     scale = 1.0,
--     mode = { width = 1920, height = 1080, refresh = 60.0 },
--     position = { x = 0, y = 0 },
--     variable_refresh_rate = false,
-- })

-- ============================================================================
-- ENVIRONMENT VARIABLES
-- ============================================================================

niri.config.environment:add("NIXOS_OZONE_WL", "1")
niri.config.environment:add("QT_QPA_PLATFORM", "wayland")

-- ============================================================================
-- NAMED WORKSPACES
-- ============================================================================

-- niri.config.workspaces:add({ name = "browser" })
-- niri.config.workspaces:add({ name = "code" })
-- niri.config.workspaces:add({ name = "terminal" })

-- ============================================================================
-- WINDOW RULES
-- ============================================================================

-- Float dialog windows
niri.config.window_rules:add({
    matches = {
        { title = ".*[Dd]ialog.*", is_regex = true },
        { title = ".*[Pp]references.*", is_regex = true },
    },
    open_floating = true,
})

-- Picture-in-Picture
niri.config.window_rules:add({
    matches = { { title = "Picture-in-Picture" } },
    open_floating = true,
    default_floating_position = { x = 20, y = 20, relative_to = "bottom-right" },
})

-- ============================================================================
-- KEY BINDINGS
-- ============================================================================

-- Clear default bindings (optional)
-- niri.config.binds:clear()

-- Window management
niri.config.binds:add("Mod+Q", niri.action:close_window())
niri.config.binds:add("Mod+Left", niri.action:focus_column_left())
niri.config.binds:add("Mod+Right", niri.action:focus_column_right())
niri.config.binds:add("Mod+Up", niri.action:focus_window_up())
niri.config.binds:add("Mod+Down", niri.action:focus_window_down())
niri.config.binds:add("Mod+Shift+Left", niri.action:move_column_left())
niri.config.binds:add("Mod+Shift+Right", niri.action:move_column_right())
niri.config.binds:add("Mod+Shift+Up", niri.action:move_window_up())
niri.config.binds:add("Mod+Shift+Down", niri.action:move_window_down())

-- Workspaces
niri.config.binds:add("Mod+Page_Up", niri.action:focus_workspace_up())
niri.config.binds:add("Mod+Page_Down", niri.action:focus_workspace_down())
niri.config.binds:add("Mod+Shift+Page_Up", niri.action:move_window_to_workspace_up())
niri.config.binds:add("Mod+Shift+Page_Down", niri.action:move_window_to_workspace_down())

-- Launchers
niri.config.binds:add("Mod+Return", niri.action:spawn({"alacritty"}))
niri.config.binds:add("Mod+D", niri.action:spawn({"fuzzel"}))

-- Layout
niri.config.binds:add("Mod+F", niri.action:fullscreen_window())
niri.config.binds:add("Mod+V", niri.action:toggle_window_floating())
niri.config.binds:add("Mod+C", niri.action:center_column())
niri.config.binds:add("Mod+W", niri.action:toggle_column_tabbed())

-- Screenshots
niri.config.binds:add("Print", niri.action:screenshot())
niri.config.binds:add("Mod+Print", niri.action:screenshot_window())

-- System
niri.config.binds:add("Mod+Shift+E", niri.action:quit())
niri.config.binds:add("Mod+Shift+Slash", niri.action:show_hotkey_overlay())
niri.config.binds:add("Mod+Tab", niri.action:toggle_overview())

-- Media keys (with allow_when_locked)
niri.config.binds:add("XF86AudioRaiseVolume", {
    action = niri.action:spawn({"wpctl", "set-volume", "@DEFAULT_AUDIO_SINK@", "5%+"}),
    allow_when_locked = true,
})
niri.config.binds:add("XF86AudioLowerVolume", {
    action = niri.action:spawn({"wpctl", "set-volume", "@DEFAULT_AUDIO_SINK@", "5%-"}),
    allow_when_locked = true,
})
niri.config.binds:add("XF86AudioMute", {
    action = niri.action:spawn({"wpctl", "set-mute", "@DEFAULT_AUDIO_SINK@", "toggle"}),
    allow_when_locked = true,
})

-- ============================================================================
-- STARTUP APPLICATIONS
-- ============================================================================

niri.config.spawn_at_startup = {
    {"waybar"},
    {"mako"},
}

-- ============================================================================
-- EVENT HANDLERS (optional)
-- ============================================================================

-- Log window focus changes
-- niri.events:on("window:focus", function(ev)
--     niri.utils.log("Focused: " .. (ev.title or "untitled"))
-- end)

-- Apply configuration
niri.config:apply()
```

---

## Appendix B: Version Compatibility Matrix

> **For LLM Agents**: Check feature availability before using.

| Feature | Added | Stable | Notes |
|---------|-------|--------|-------|
| Core Lua API | 25.01 | Yes | `niri.config`, `niri.action`, `niri.events` |
| Collection proxies | 25.01 | Yes | `:add()`, `:remove()`, `:list()`, `:clear()` |
| Timer API | 25.01 | Yes | `niri.loop.new_timer()` |
| REPL | 25.01 | Yes | `niri msg lua` |
| `spawn_sh` | 25.05 | Yes | Shell command spawning |
| `trackpoint` input | 25.05 | Yes | Trackpoint device configuration |
| `numlock` setting | 25.05 | Yes | Keyboard numlock on startup |
| `mod_key` override | 25.05 | Yes | Custom modifier key |
| `hide_when_typing` | 25.05 | Yes | Cursor hiding option |
| `backdrop_color` | 25.05 | Yes | Output backdrop color |
| `modeline` | 25.05 | Yes | Custom video mode |
| `geometry_corner_radius` | 25.05 | Yes | Window rule property |
| `toggle_keyboard_shortcuts_inhibit` | 25.05 | Yes | Action |
| `expand_column_to_available_width` | 25.05 | Yes | Action |
| `always_center_single_column` | 25.05 | Yes | Layout option |
| `tab_indicator.position` | 25.05 | Yes | Tab indicator placement |
| `shadow.corner_radius` | 25.05 | Yes | Shadow corner radius |
| Custom animation shaders | 25.05 | Yes | GLSL shaders |
| `recent_windows` config | 25.11 | Yes | Recent windows UI |
| `default_column_display` | 25.11 | Yes | Normal/tabbed default |
| Per-window decoration overrides | 25.11 | Yes | focus_ring, border, shadow per rule |
| `scroll_factor` window rule | 25.11 | Yes | Per-window scroll sensitivity |
| `baba_is_float` | 25.11 | Yes | Float behavior for tiled |
| `center_visible_columns` | 25.11 | Yes | Action |
| `toggle_column_tabbed_display` | 25.11 | Yes | Action |
| Window cast actions | 25.11 | Yes | Dynamic cast target |
| Urgency actions | 25.11 | Yes | `toggle_window_urgent` |
| `config:reload` event | 25.11 | Yes | Config reload notification |
| `overview:open/close` events | 25.11 | Yes | Overview state events |
| Plugin system | 25.11 | Partial | Architecture complete, not integrated |

### Deprecations

| Feature | Deprecated | Removed | Replacement |
|---------|------------|---------|-------------|
| None yet | - | - | - |

### Breaking Changes

| Version | Change | Migration |
|---------|--------|-----------|
| None yet | - | - |

---

## Appendix C: Test Coverage Matrix

> **For LLM Agents**: Current test coverage status. Gaps indicate areas needing attention.

### Module Coverage

| Module | Unit Tests | Integration Tests | Snapshot Tests | Coverage % | Notes |
|--------|------------|-------------------|----------------|------------|-------|
| `config_wrapper.rs` | Partial | No | Yes | ~40% | Needs field validation tests |
| `action_proxy.rs` | Partial | No | No | ~30% | Many actions untested |
| `event_data.rs` | Yes | No | No | ~60% | Payload types covered |
| `state_query.rs` | Partial | No | Yes | ~50% | Query return types covered |
| `repl.rs` | Yes | Yes | No | ~70% | Command execution covered |
| `timer_api.rs` | Yes | No | No | ~80% | Timer lifecycle tested |

### Test Priorities

| Priority | Area | Current State | Target |
|----------|------|---------------|--------|
| P0 | Config field assignment | 10 tests | 50+ tests |
| P0 | Collection CRUD operations | 5 tests | 30+ tests |
| P0 | Action execution | 15 tests | 90+ tests (all actions) |
| P1 | Event handler lifecycle | 3 tests | 15+ tests |
| P1 | State query accuracy | 8 tests | 20+ tests |
| P2 | Error handling paths | 2 tests | 20+ tests |
| P2 | Edge cases | 5 tests | 30+ tests |

### Missing Test Categories

- [ ] Config validation (invalid values, missing fields)
- [ ] Memory management (handler cleanup)
- [ ] Concurrent access (multiple scripts)
- [ ] Performance benchmarks
- [ ] Fuzz testing (random inputs)
- [ ] Error message quality

---

## Appendix D: Code Review Checklist

> **For LLM Agents and Reviewers**: Use this checklist when reviewing niri-lua code.

### Security Review

- [ ] **Sandbox integrity**: No paths to escape Lua sandbox
- [ ] **File system access**: Lua cannot access arbitrary files
- [ ] **Network access**: Lua cannot make network requests
- [ ] **Process spawning**: Only via sanctioned `spawn` actions
- [ ] **Environment access**: Limited to `niri.config.environment`
- [ ] **Memory limits**: Script memory usage is bounded
- [ ] **Execution limits**: Script CPU time is bounded

### Memory Safety

- [ ] **Lua references**: All `LuaRegistryKey` properly dropped
- [ ] **Callback cleanup**: Event handlers removed on script reload
- [ ] **Cyclic references**: No Lua↔Rust reference cycles
- [ ] **Large allocations**: Bounded and validated
- [ ] **String handling**: No unbounded string concatenation

### Type Safety

- [ ] **Lua→Rust conversion**: All `from_lua` handle type errors
- [ ] **Rust→Lua conversion**: All `to_lua` produce valid Lua values
- [ ] **Nil handling**: Nullable fields handle `nil` correctly
- [ ] **Table structure**: Expected keys validated
- [ ] **Enum values**: Invalid enums rejected with clear error

### Error Handling

- [ ] **No panics**: Use `Result`/`Option`, not `unwrap()` in user paths
- [ ] **Error messages**: Include context (field name, expected type)
- [ ] **Recovery**: Errors don't leave system in bad state
- [ ] **Logging**: Errors logged for debugging

### Performance

- [ ] **Hot paths**: No allocations in per-frame code
- [ ] **Caching**: Expensive computations cached
- [ ] **Lazy evaluation**: Don't compute unused values
- [ ] **Batch operations**: Prefer batch over single-item ops

### API Consistency

- [ ] **Naming**: Follows existing patterns (snake_case, verb_noun)
- [ ] **Return types**: Consistent with similar APIs
- [ ] **Error behavior**: Matches documented contracts
- [ ] **Documentation**: Doc comments on public items

### Testing

- [ ] **Unit tests**: New code has unit tests
- [ ] **Edge cases**: Tests cover boundary conditions
- [ ] **Error paths**: Tests verify error handling
- [ ] **Snapshots**: Config serialization has snapshots

### Documentation

- [ ] **Spec updated**: LUA_SPECIFICATION.md reflects changes
- [ ] **Examples**: Non-obvious usage has examples
- [ ] **Since annotations**: New features marked with version
- [ ] **Type annotations**: EmmyLua types updated

---

## Appendix E: Refactor History

> **Purpose**: Document significant refactoring decisions and cleanup work for future reference.

### December 2025 Cleanup

A comprehensive code quality analysis was performed on the niri-lua crate using redundancy-checker and yagni-checker tools. The following changes were made:

#### Removed (Dead Code)
| File | LOC | Reason |
|------|-----|--------|
| `event_emitter.rs` | 284 | Duplicate implementation; `events_proxy.rs` is the active event system |

#### Created (Consolidation)
| File | LOC | Purpose |
|------|-----|---------|
| `parse_utils.rs` | 122 | Shared size/position parsing extracted from multiple files |

#### Fixed (Code Quality)
- Replaced defensive fallback code with explicit `.expect()` calls
- Removed trivial assertions (`assert!(true)`, `assert_eq!(x, false)`)

#### Simplified (December 2025)

The plugin/module system was simplified to follow the Neovim model more closely:

| Change | Before | After |
|--------|--------|-------|
| `plugin_system.rs` | 716 LOC with PluginManager, metadata, lifecycle | Deleted - over-engineered |
| `module_loader.rs` | 277 LOC with custom require override | Deleted - plugins use standard Lua `require()` |

The intended approach is to use Luau's standard `require()` semantics. Users can load modules relative to their config file or via absolute paths.

#### Intentionally Kept (Not Dead Code)

| File | LOC | Justification |
|------|-----|---------------|
| `ipc_repl.rs` | 78 | Neovim-style `:lua` command via IPC (`niri msg lua "code"`) |
| `config_dirty.rs` | 161 | Granular change tracking enables future partial config reload optimization |
| `lua_types.rs` | 395 | Type definitions required for config/runtime APIs |

#### Design Decision: Keep Granular Dirty Flags

The `config_dirty.rs` module tracks 21 individual config section flags, though currently only `.any()` is called. This was intentionally kept because:

1. **Future optimization**: Enables partial config reload (only re-apply changed sections)
2. **Clean implementation**: Well-structured code with minimal maintenance burden
3. **Debugging value**: Individual flags aid in debugging config change propagation

#### Design Decision: Simple Module Loading

Plugins follow the Neovim model - pure Lua modules loaded via standard `require()`:

1. **Standard Lua semantics**: `require("foo.bar")` works exactly as expected
2. **Relative requires**: `require("./utils")` loads relative to the current file
3. **No plugin manager**: Users load plugins explicitly via `require()` in their config
4. **User-managed paths**: Users can set up their own module directories

### Future Optimization Opportunities

The following items were identified during analysis but deferred as low-priority optimizations:

#### P2: Config Conversion Pipeline Refactoring

**Files involved**:
- `config_wrapper.rs` (~3,200 LOC)
- `extractors.rs` (~1,600 LOC)
- `config_api.rs` (~950 LOC)

**Current flow**: Lua table → JSON → HashMap → niri Config (triple conversion)

**Potential optimization**: Direct Lua → Config conversion via a `ConfigPatch` trait pattern

**Trade-offs**:
- Current approach works correctly and is well-tested
- JSON intermediary simplifies debugging (human-readable)
- Refactoring effort estimated at 40-60 hours
- Performance impact is minimal for config loading (one-time operation)

**Decision**: Defer until performance profiling indicates this is a bottleneck.

#### P3: Minor Optimizations

**Snapshot test coverage** (`src/snapshots/`):
- Consider auditing for tests that verify implementation details vs. behavior
- Low priority; current tests provide good regression coverage

**String cloning patterns** (`collections.rs`):
- Some string clones in loops could potentially be avoided
- Profile before optimizing; impact is likely negligible
