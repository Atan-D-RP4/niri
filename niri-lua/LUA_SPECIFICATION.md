# Niri Lua Specification

Comprehensive specification for niri's Lua scripting system. This document covers the complete API surface, architecture, and implementation details.

**Current Version**: 25.11
**Minimum Supported**: 25.01
**Spec Revision**: 2.0 (optimized for agentic LLM consumption)

## Table of Contents

1. [Quick Reference](#quick-reference) *(NEW - single-page cheat sheet)*
2. [Overview](#overview)
3. [API Patterns](#api-patterns) *(NEW - decision tree for API usage)*
4. [Schema Reference](#schema-reference) *(NEW - machine-readable types)*
5. [Configuration API](#configuration-api)
6. [Runtime State API](#runtime-state-api)
7. [Event System](#event-system) *(with typed payloads)*
8. [Action System](#action-system) *(with typed signatures)*
9. [Timer API](#timer-api)
10. [Validation Rules](#validation-rules) *(NEW)*
11. [Error Reference](#error-reference) *(NEW)*
12. [Cookbook](#cookbook) *(NEW - common recipes)*
13. [REPL](#repl)
14. [Plugin System](#plugin-system)
15. [Review & Testing](#review-and-testing) *(NEW)*
16. [Appendices](#appendices)
    - [A: Canonical Reference Config](#appendix-a-canonical-reference-config)
    - [B: Version Compatibility Matrix](#appendix-b-version-compatibility-matrix)
    - [C: Test Coverage Matrix](#appendix-c-test-coverage-matrix)
    - [D: Code Review Checklist](#appendix-d-code-review-checklist)

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
| `niri.utils` | Logging utilities | Methods |
| `niri.loop` | Timer/scheduling | Methods |

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
local windows = niri.state.windows        -- {id, app_id, title, workspace_id, ...}[]
local workspaces = niri.state.workspaces  -- {id, name, output, is_active, ...}[]
local outputs = niri.state.outputs        -- {name, make, model, width, height, ...}[]
local layouts = niri.state.keyboard_layouts -- {names, current_idx}
local focused = niri.state.focused_window -- {id, app_id, title} | nil
```

### Events (14 total)

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
niri.action.close_window()
niri.action.fullscreen_window()
niri.action.focus_window(id)              -- id: integer
niri.action.focus_window_up()
niri.action.focus_window_down()

-- Workspace
niri.action.focus_workspace(ref)          -- ref: integer | string
niri.action.move_window_to_workspace(ref)

-- Spawn
niri.action.spawn({"cmd", "arg1"})        -- array of strings
niri.action.spawn_sh("shell command")     -- shell string

-- Layout
niri.action.toggle_window_floating()
niri.action.toggle_column_tabbed()
niri.action.set_column_width({proportion = 0.5})

-- System
niri.action.quit()
niri.action.power_off_monitors()
niri.action.screenshot()
```

### Timers

```lua
local timer = niri.loop.new_timer(1000, function() end)  -- one-shot, ms
local timer = niri.loop.new_timer(1000, function() end, true)  -- repeating
timer:close()  -- cancel
```

### Logging

```lua
niri.utils.log("info message")
niri.utils.debug("debug message")
niri.utils.warn("warning message")
niri.utils.error("error message")
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
- 14 events wired across 5 categories

---

## Vision

Niri's Lua system is foundational infrastructure for transforming niri from a standalone compositor into a **complete desktop environment framework**—similar to how Neovim serves as a base that becomes a full IDE through Lua configuration (LazyVim, LunarVim, AstroNvim).

### The Desktop Environment Framework

The long-term vision comprises two complementary crates:

| Crate | Purpose | Status |
|-------|---------|--------|
| **niri-lua** | Configuration, runtime state, events, actions | Implemented |
| **niri-ui** | Smithay-native widget toolkit for shell components | Design phase |

Together, these enable building full desktop shells comparable to:
- **AwesomeWM**: Lua-configured window manager with built-in widgets
- **QtQuick/Shell projects**: Noctalia Shell, DankMaterialShell
- **KDE Plasma**: Full desktop environment with panels, widgets, system integration

### What niri-lua Provides Today

1. **Complete Configuration**: Full KDL parity—every config option is scriptable
2. **Runtime State Queries**: Access to windows, workspaces, outputs, keyboard layouts
3. **Event System**: 14 compositor events for reactive programming
4. **Action System**: ~90 actions for controlling the compositor
5. **Timers**: Deferred and repeating callbacks for dynamic behavior
6. **REPL**: Interactive Lua console for debugging and exploration

### What niri-ui Will Add

The planned `niri-ui` crate (see [NIRI_UI_SPECIFICATION.md](../docs/NIRI_UI_SPECIFICATION.md)) will enable:

- **Status bars and panels**: Top/bottom bars with workspaces, clock, system info
- **Application launchers**: dmenu/rofi-style launchers built in Lua
- **Notification centers**: OSD and notification management
- **System trays**: SNI (StatusNotifierItem) protocol support
- **Custom overlays**: Volume/brightness indicators, window switchers
- **D-Bus integration**: Full system service access from Lua

### The Neovim Analogy

| Neovim | Niri |
|--------|------|
| Base editor | Base compositor |
| Lua API | niri-lua |
| UI plugins (telescope, lualine) | niri-ui widgets |
| Distribution (LazyVim) | Community "niri distros" |

Just as Neovim users can build entirely custom editing experiences through Lua, niri users will be able to build entirely custom desktop experiences—without writing any Rust.

### Design Principles for the Framework

1. **Compositor stability is paramount**: UI crashes must never take down the compositor
2. **Smithay-native rendering**: No external GUI dependencies; use niri's existing Cairo/Pango → GlowRenderer pipeline
3. **Minimal compositor changes**: niri-ui lives in a separate crate with <50 lines of integration code
4. **Lua-first API**: All shell components are defined declaratively in Lua tables
5. **Plugin ecosystem ready**: Sandboxed plugins with lifecycle management, dependencies, and permissions

### Current Status

- ✅ **niri-lua**: Complete and production-ready
  - Configuration API: Full KDL parity (input, outputs, layout, binds, window_rules, etc.)
  - Runtime API: Windows, workspaces, outputs, keyboard layouts
  - Events: 14 compositor events (Window, Workspace, Monitor, Layout, Config)
  - Actions: ~90 actions
  - REPL: Interactive development

- ✅ **Compositor Integration**: Complete
  - `src/lua_integration.rs`: Consolidated Lua setup (~12 lines in main.rs)
  - `src/lua_event_hooks.rs`: Extension traits for event emission
  - Centralized event emission in refresh cycle (not scattered call sites)

- 🔄 **niri-ui**: Design phase
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

### Config Conversion: JSON Intermediary Format

**Decision:** Use `serde_json::Value` as an intermediary when converting Lua tables to Config structs.

**Rationale:**
- Simpler implementation than direct Lua-to-Config conversion
- Leverages existing serde infrastructure
- Easier debugging (can log JSON)
- Trade-off: Additional serialization step

**Noted as potential optimization:** Direct Lua-to-Config conversion could be more efficient but requires more complex code.

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
| Config convert | JSON intermediary | Direct | Simpler implementation |
| Type defs | EmmyLua | Luau native | LSP compatibility |
| Raw keys | Excluded | Included | Security, performance |

---

## Architecture

### Tiered Implementation

The Lua system is organized into tiers of increasing functionality:

| Tier | Feature | Status |
|------|---------|--------|
| 1 | Module system, plugin discovery, event emitter | Complete |
| 2 | Configuration API with full KDL parity | Complete |
| 3 | Runtime state queries (windows, workspaces, outputs) | Complete |
| 4 | Event system with compositor integration | Complete |
| 5 | Plugin ecosystem (sandboxing, lifecycle) | Partial |
| 6 | Developer experience (REPL, docs, types) | Complete |

### Core Components

```
niri-lua/src/
├── lib.rs              # Public API exports
├── runtime.rs          # LuaRuntime core (timeout, scheduler, timer management)
├── config.rs           # Config struct definitions
├── config_api.rs       # niri.config API entry point
├── config_wrapper.rs   # Config proxies with section access
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
├── plugin_system.rs    # Plugin discovery, lifecycle, sandboxing (Tier 5)
├── module_loader.rs    # Custom Lua module resolution (Tier 5)
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


WHEN using niri.action.*:
│
├─ Action takes NO arguments:
│   niri.action.close_window()
│   niri.action.quit()
│
├─ Action takes a SINGLE primitive:
│   niri.action.focus_window(id)           -- integer
│   niri.action.focus_workspace(ref)       -- integer | string
│
├─ Action takes an ARRAY:
│   niri.action.spawn({"cmd", "arg1"})     -- string[]
│
└─ Action takes a TABLE:
    niri.action.set_column_width({proportion = 0.5})
    niri.action.set_window_height({fixed = 600})


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
    local windows = niri.state.windows         -- array
    local focused = niri.state.focused_window  -- table | nil
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
niri.config.binds:add("Mod+Q", niri.action.close_window)  -- ❌ Missing ()

-- RIGHT: Call the action factory
niri.config.binds:add("Mod+Q", niri.action.close_window())  -- ✓

-- WRONG: spawn with string
niri.action.spawn("alacritty")  -- ❌ Expects array

-- RIGHT: spawn with array
niri.action.spawn({"alacritty"})  -- ✓

-- RIGHT: spawn_sh with string
niri.action.spawn_sh("alacritty --working-directory ~")  -- ✓
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
niri.config.binds:add("Mod+Return", niri.action.spawn({"alacritty"}))
niri.config.binds:add("Mod+Q", niri.action.close_window())
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

### Collection Proxies

Fields marked as `collection` in the table above use special proxy objects that provide CRUD operations. This is the pattern for managing arrays/maps of items like bindings, window rules, outputs, and workspaces.

```lua
-- Available collection proxy methods:

-- :add(item) - Add an item to the collection
niri.config.binds:add("Mod+Q", niri.action.close_window())
niri.config.window_rules:add({ matches = { { app_id = "firefox" } }, open_floating = true })
niri.config.outputs:add({ name = "eDP-1", scale = 1.5 })

-- :list() - Get all items in the collection
local all_binds = niri.config.binds:list()
local all_rules = niri.config.window_rules:list()

-- :get(key) - Get a specific item by key/name
local output = niri.config.outputs:get("eDP-1")

-- :remove(key) - Remove a specific item
niri.config.binds:remove("Mod+Q")
niri.config.outputs:remove("DP-1")

-- :clear() - Remove all items from the collection
niri.config.binds:clear()
niri.config.window_rules:clear()

-- :set(table) - Replace entire collection (for table-like configs)
niri.config.environment:set({
    GTK_THEME = "Adwaita:dark",
    QT_QPA_PLATFORM = "wayland",
})

-- Collections available:
-- - niri.config.binds (keybindings)
-- - niri.config.outputs (monitors)
-- - niri.config.window_rules (window matching rules)
-- - niri.config.layer_rules (layer surface rules)
-- - niri.config.workspaces (named workspaces)
-- - niri.config.environment (environment variables)
```

### Input Configuration

```lua
niri.config.input = {
    keyboard = {
        xkb = {
            layout = "us,de",
            variant = "",
            options = "grp:alt_shift_toggle",
            model = "",
            rules = "",
        },
        repeat_delay = 600,
        repeat_rate = 25,
        track_layout = "global", -- "global" | "window"
        numlock = true, -- Enable NumLock on startup (Since 25.05)
    },
    touchpad = {
        tap = true,
        dwt = true,  -- disable while typing
        dwtp = true, -- disable while trackpointing
        natural_scroll = true,
        accel_speed = 0.0,
        accel_profile = "adaptive", -- "adaptive" | "flat"
        tap_button_map = "left-right-middle",
        click_method = "button-areas", -- "button-areas" | "clickfinger"
        scroll_method = "two-finger", -- "two-finger" | "edge" | "on-button-down"
        disabled = false,
    },
    mouse = {
        natural_scroll = false,
        accel_speed = 0.0,
        accel_profile = "flat",
        scroll_button = 274, -- middle button
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
    trackball = {
        natural_scroll = false,
        accel_speed = 0.0,
        accel_profile = "flat",
        scroll_method = "on-button-down",
        scroll_button = 274,
        disabled = false,
    },
    tablet = {
        map_to_output = "eDP-1",
    },
    touch = {
        map_to_output = "eDP-1",
    },
    -- Modifier key override (Since 25.05)
    mod_key = "Super",  -- Override the Mod key (default: Super)
    mod_key_nested = "Super",  -- Mod key when running nested
    power_key_handling = "suspend", -- "suspend" | "ignore"
    disable_power_key_handling = false,
    warp_mouse_to_focus = false,
    focus_follows_mouse = false, -- true | false | "max-scroll-amount"
    workspace_auto_back_and_forth = false,
}
```

### Output Configuration

```lua
-- Outputs use collection proxy methods
niri.config.outputs:add({
    name = "eDP-1",
    off = false, -- Disable this output entirely
    mode = {
        width = 1920,
        height = 1080,
        refresh = 60.0,
    },
    -- OR use modeline string (Since 25.05)
    -- modeline = "173.00 1920 2048 2248 2576 1080 1083 1088 1120 -hsync +vsync",
    scale = 1.0,
    transform = "normal", -- "normal" | "90" | "180" | "270" | "flipped" | "flipped-90" | "flipped-180" | "flipped-270"
    position = { x = 0, y = 0 },
    variable_refresh_rate = false, -- true | false | "on-demand" | "hint"
    background_color = "#000000",
    backdrop_color = "#000000", -- Color behind windows (Since 25.05)
    focus_at_startup = false, -- Focus this output on startup
})

-- Output can have per-output layout overrides (Since 25.11)
niri.config.outputs:add({
    name = "DP-1",
    scale = 2.0,
    -- Layout overrides for this output only
    default_column_width = { proportion = 0.5 },
    focus_ring = { width = 2 },
})
```

### Layout Configuration

```lua
niri.config.layout = {
    gaps = 16,
    struts = {
        left = 0,
        right = 0,
        top = 0,
        bottom = 0,
    },
    center_focused_column = "never", -- "never" | "always" | "on-overflow"
    always_center_single_column = false, -- Center even single columns (Since 25.05)
    background_color = nil, -- Fallback color when no wallpaper (Since 25.05)
    default_column_display = "normal", -- "normal" | "tabbed" (Since 25.11)
    empty_workspace_above_first = false, -- Keep empty workspace above first (Since 25.05)
    preset_column_widths = {
        { proportion = 1/3 },
        { proportion = 1/2 },
        { proportion = 2/3 },
    },
    default_column_width = { proportion = 0.5 },
    preset_window_heights = {
        { proportion = 1/3 },
        { proportion = 1/2 },
        { proportion = 2/3 },
    },
    focus_ring = {
        off = false, -- Disable focus ring
        width = 4,
        active_color = "#7fc8ff",
        inactive_color = "#505050",
        -- OR use gradient:
        active_gradient = {
            from = "#80c8ff",
            to = "#bbddff",
            angle = 45,
            relative_to = "workspace-view", -- "workspace-view" | "window"
            in_ = "srgb", -- Color interpolation space (Since 25.05)
        },
    },
    border = {
        off = false,
        width = 4,
        active_color = "#ffc87f",
        inactive_color = "#505050",
        -- Also supports gradients like focus_ring
    },
    shadow = {
        on = true, -- Enable shadows
        softness = 30,
        spread = 5,
        offset = { x = 0, y = 5 },
        color = "#00000070",
        inactive_color = "#00000040",
        -- Per-corner radius (Since 25.05)
        corner_radius = 12.0,
    },
    tab_indicator = {
        off = false,
        hide_when_single_tab = true,
        place_within_column = false, -- Place inside column bounds (Since 25.05)
        gap = 10.0,
        width = 4.0,
        length = { proportion = 0.3 },
        corner_radius = 8.0,
        active_color = "#ffc87f",
        inactive_color = "#505050",
        position = "left", -- "left" | "right" | "top" | "bottom" (Since 25.05)
    },
    insert_hint = {
        off = false,
        color = "#ffc87f80",
        -- Also supports gradient
    },
}
```

### Key Bindings

Key bindings use collection proxies with the `:add()` method:

```lua
-- Simple bindings
niri.config.binds:add("Mod+Return", niri.action.spawn({"alacritty"}))
niri.config.binds:add("Mod+Q", niri.action.close_window())

-- Bindings with modifiers
niri.config.binds:add("Mod+Shift+E", niri.action.quit())
niri.config.binds:add("Mod+Ctrl+L", niri.action.spawn({"swaylock"}))

-- Allow when locked (using options table)
niri.config.binds:add("XF86AudioRaiseVolume", {
    action = niri.action.spawn({"wpctl", "set-volume", "@DEFAULT_AUDIO_SINK@", "5%+"}),
    allow_when_locked = true,
})

-- Cooldown (prevent rapid re-triggering)
niri.config.binds:add("Mod+Tab", {
    action = niri.action.focus_window_down_or_column_right(),
    cooldown_ms = 150,
})

-- Repeat on hold (Since 25.05)
niri.config.binds:add("Mod+Left", {
    action = niri.action.focus_column_left(),
    repeat_ = true,  -- Repeat while held
})

-- Allow inhibiting (default true, set false to override app inhibit)
niri.config.binds:add("Mod+Escape", {
    action = niri.action.toggle_keyboard_shortcuts_inhibit(),
    allow_inhibiting = false,
})

-- Custom hotkey overlay title
niri.config.binds:add("Mod+D", {
    action = niri.action.spawn({"fuzzel"}),
    hotkey_overlay_title = "App Launcher",
})

-- Mouse button bindings (Since 25.05)
niri.config.binds:add("Mod+MouseLeft", niri.action.move_window())
niri.config.binds:add("Mod+MouseRight", niri.action.resize_window())
niri.config.binds:add("MouseForward", niri.action.focus_workspace_down())
niri.config.binds:add("MouseBack", niri.action.focus_workspace_up())

-- Scroll wheel bindings
niri.config.binds:add("Mod+WheelScrollDown", niri.action.focus_workspace_down())
niri.config.binds:add("Mod+WheelScrollUp", niri.action.focus_workspace_up())

-- Touchpad scroll bindings (high-resolution)
niri.config.binds:add("Mod+TouchpadScrollDown", {
    action = niri.action.focus_workspace_down(),
    cooldown_ms = 150,
})

-- Clear all bindings
niri.config.binds:clear()

-- List all bindings
local all_binds = niri.config.binds:list()

-- Remove specific binding
niri.config.binds:remove("Mod+Q")
```

### Window Rules

```lua
-- Window rules use collection proxies
niri.config.window_rules:add({
    matches = {
        { app_id = "firefox" },
        { app_id = "chromium" },
    },
    default_column_width = { proportion = 0.7 },
})

-- Regex matching
niri.config.window_rules:add({
    matches = {
        { app_id = ".*dialog.*", is_regex = true },
    },
    open_floating = true,
})

-- Floating position
niri.config.window_rules:add({
    matches = {
        { title = "Picture-in-Picture" },
    },
    open_floating = true,
    default_floating_position = { x = 20, y = 20, relative_to = "top-right" },
})

-- Full match criteria (Since 25.05+)
niri.config.window_rules:add({
    matches = {
        {
            app_id = "org.gnome.Nautilus",
            title = ".*Downloads.*",
            is_regex = true,
            -- State-based matching
            is_focused = true,
            is_active = false,
            is_active_in_column = true,
            is_floating = false,
            is_window_cast_target = false,
            is_urgent = false,
            at_startup = true, -- Only match at compositor startup
        },
    },
    excludes = {
        { title = "Preferences" }, -- Exclude preferences dialogs
    },
    -- Window properties
    open_on_output = "eDP-1",
    open_on_workspace = "main",
    open_maximized = false,
    open_fullscreen = false,
    open_floating = false,
    default_column_width = { proportion = 0.5 },
    default_window_height = { fixed = 600 },

    -- Visual overrides (Since 25.05)
    opacity = 0.95,
    draw_border_with_background = true,
    geometry_corner_radius = 12.0, -- Override CSD corner radius
    clip_to_geometry = true, -- Clip to reported geometry

    -- Per-window decoration overrides (Since 25.11)
    focus_ring = {
        off = false,
        width = 2,
        active_color = "#ff0000",
    },
    border = {
        off = true, -- Disable border for this window
    },
    shadow = {
        on = true,
        color = "#00000080",
    },
    tab_indicator = {
        off = false,
        active_color = "#00ff00",
    },

    -- Behavior overrides
    block_out_from = "screencast", -- "screencast" | "screen-capture"
    scroll_factor = 1.0, -- Mouse scroll sensitivity (Since 25.11)
    baba_is_float = false, -- Treat tiled as floating for focus (Since 25.11)
    tiled_state = nil, -- Force tiled state: true | false | nil (Since 25.11)

    -- Size constraints
    min_width = 100,
    max_width = 1000,
    min_height = 100,
    max_height = 800,

    -- Column display (Since 25.11)
    default_column_display = "tabbed", -- "normal" | "tabbed"
})
```

### Animations

```lua
niri.config.animations = {
    off = false, -- Disable all animations
    slowdown = 1.0,  -- Global slowdown factor (higher = slower)

    -- All 11 animation types:
    workspace_switch = {
        spring = {
            damping_ratio = 1.0,
            stiffness = 1000,
            epsilon = 0.0001,
        },
    },
    window_open = {
        easing = {
            duration_ms = 150,
            curve = "ease-out-expo",
        },
        -- OR custom cubic bezier:
        -- easing = {
        --     duration_ms = 200,
        --     curve = { 0.2, 0.0, 0.0, 1.0 },
        -- },
    },
    window_close = {
        easing = {
            duration_ms = 100,
            curve = "ease-out-quad",
        },
    },
    window_movement = {
        spring = {
            damping_ratio = 1.0,
            stiffness = 800,
            epsilon = 0.0001,
        },
    },
    window_resize = {
        spring = {
            damping_ratio = 1.0,
            stiffness = 800,
            epsilon = 0.0001,
        },
    },
    horizontal_view_movement = {
        spring = {
            damping_ratio = 1.0,
            stiffness = 800,
            epsilon = 0.0001,
        },
    },
    config_notification_open_close = {
        spring = {
            damping_ratio = 0.6,
            stiffness = 1000,
            epsilon = 0.001,
        },
    },
    screenshot_ui_open = {
        easing = {
            duration_ms = 200,
            curve = "ease-out-quad",
        },
    },
    overview_open_close = {
        easing = {
            duration_ms = 200,
            curve = "ease-out-expo",
        },
    },
    exit_confirmation_open_close = {
        spring = {
            damping_ratio = 0.8,
            stiffness = 1000,
            epsilon = 0.001,
        },
    },
    recent_windows_close = { -- Since 25.11
        easing = {
            duration_ms = 150,
            curve = "ease-out-quad",
        },
    },

    -- Custom shaders (Since 25.05)
    shaders = {
        window_open = [[
            // Custom GLSL shader for window open animation
            vec4 open_color(vec3 coords_geo, vec3 size_geo) {
                // ... shader code
                return color;
            }
        ]],
        window_close = nil, -- Use default
    },
}

-- Available easing curves:
-- "ease-out-quad", "ease-out-cubic", "ease-out-expo", "ease-out-quint"
-- "linear"
-- Or custom bezier: { x1, y1, x2, y2 }
```

### Named Workspaces

```lua
-- Named workspaces use collection proxies
niri.config.workspaces:add({
    name = "browser",
    open_on_output = "eDP-1", -- Which output to open on
})

niri.config.workspaces:add({
    name = "terminal",
    open_on_output = "DP-1",
    -- Per-workspace layout overrides (Since 25.11)
    default_column_width = { proportion = 0.5 },
    focus_ring = { width = 2 },
})

-- List all named workspaces
local workspaces = niri.config.workspaces:list()
```

### Recent Windows (Since 25.11)

```lua
niri.config.recent_windows = {
    -- Highlight settings
    highlight = {
        width = 2,
        active_color = "#ffc87f",
        -- Also supports gradients
    },
    -- Preview settings
    previews = {
        background_color = "#00000080",
        border = {
            width = 1,
            active_color = "#ffffff",
            inactive_color = "#808080",
        },
    },
}
```

### Clipboard

```lua
niri.config.clipboard = {
    disable_primary = false, -- Disable primary selection clipboard
}
```

### XWayland-Satellite

```lua
niri.config.xwayland_satellite = {
    off = false, -- Disable XWayland-satellite entirely
    path = "/usr/bin/xwayland-satellite", -- Custom path (optional)
}
```

### Config Notification

```lua
niri.config.config_notification = {
    disable_failed = false, -- Disable notification on config errors
}
```

### Switch Events

```lua
niri.config.switch_events = {
    lid_close = niri.action.spawn({"swaylock"}),
    lid_open = nil, -- No action
    tablet_mode_on = nil,
    tablet_mode_off = nil,
}
```

### Layer Rules

```lua
-- Layer rules for layer-shell surfaces (panels, overlays, etc.)
niri.config.layer_rules:add({
    matches = {
        { namespace = "waybar" },
        { namespace = ".*bar.*", is_regex = true },
    },
    -- Properties
    block_out_from = "screencast", -- "screencast" | "screen-capture"
    opacity = 0.95,
    -- Geometry corner radius
    geometry_corner_radius = 0,
    -- Animation overrides
    animation = {
        easing = {
            duration_ms = 150,
            curve = "ease-out-quad",
        },
    },
})
```

### Gestures

```lua
niri.config.gestures = {
    workspace_swipe = {
        three_finger = true,
        four_finger = false,
        horizontal = true,
        distance = 400,
        natural_scroll = true,
    },
}
```

### Cursor

```lua
niri.config.cursor = {
    xcursor_theme = "Adwaita",
    xcursor_size = 24,
    hide_when_typing = false, -- Hide cursor when typing (Since 25.05)
    hide_after_inactive_ms = 0, -- Hide after inactivity (0 = never)
}
```

### Overview

```lua
niri.config.overview = {
    backdrop_color = "#00000080", -- Backdrop overlay color
}
```

### Hotkey Overlay

```lua
niri.config.hotkey_overlay = {
    skip_at_startup = false, -- Don't show overlay on startup
}
```

### Environment Variables

```lua
-- Environment variables use collection proxies
niri.config.environment:add("GTK_THEME", "Adwaita:dark")
niri.config.environment:add("QT_QPA_PLATFORM", "wayland")

-- Set multiple at once
niri.config.environment:set({
    GTK_THEME = "Adwaita:dark",
    QT_QPA_PLATFORM = "wayland",
    XCURSOR_THEME = "Adwaita",
})
```

---

## Runtime State API

The `niri.state` namespace provides read-only access to compositor state. All queries return **snapshots** (copies) of the current state to avoid deadlocks.

### Windows

```lua
-- Get all windows
local windows = niri.state.windows()
for _, win in ipairs(windows) do
    print(win.id, win.app_id, win.title)
end

-- Get focused window
local focused = niri.state.focused_window()
if focused then
    print("Focused:", focused.app_id, focused.title)
end
```

**Window properties:**
| Property | Type | Description |
|----------|------|-------------|
| `id` | number | Unique window ID |
| `app_id` | string | Application identifier |
| `title` | string | Window title |
| `workspace_id` | number | Containing workspace ID |
| `is_focused` | boolean | Whether window has focus |
| `is_floating` | boolean | Whether window is floating |

### Workspaces

```lua
local workspaces = niri.state.workspaces()
for _, ws in ipairs(workspaces) do
    print(ws.id, ws.name, ws.output, ws.is_active)
end
```

**Workspace properties:**
| Property | Type | Description |
|----------|------|-------------|
| `id` | number | Unique workspace ID |
| `idx` | number | Index on output (1-based) |
| `name` | string? | Optional workspace name |
| `output` | string | Output name |
| `is_active` | boolean | Whether workspace is active on its output |
| `is_focused` | boolean | Whether workspace has global focus |
| `active_window_id` | number? | ID of active window, if any |

### Outputs

```lua
local outputs = niri.state.outputs()
for _, out in ipairs(outputs) do
    print(out.name, out.width, out.height, out.scale)
end
```

**Output properties:**
| Property | Type | Description |
|----------|------|-------------|
| `name` | string | Output name (e.g., "eDP-1") |
| `make` | string | Monitor manufacturer |
| `model` | string | Monitor model |
| `serial` | string | Monitor serial number |
| `width` | number | Logical width in pixels |
| `height` | number | Logical height in pixels |
| `refresh` | number | Refresh rate in Hz |
| `scale` | number | Scale factor |
| `transform` | string | Applied transform |
| `is_enabled` | boolean | Whether output is enabled |

---

## Event System

The `niri.events` namespace provides pub/sub event handling for compositor events.

> **For LLM Agents**: This section documents 14 events with fully typed payloads.

### API Methods

```lua
-- Single event subscription
---@param event string Event name (e.g., "window:open")
---@param handler function(payload: table) Callback function
---@return integer handler_id Unique ID for unsubscribing
local id = niri.events:on("window:open", function(event)
    print("Window opened:", event.app_id, event.title)
end)

-- Multi-event subscription (vim-style autocmd pattern)
-- Same callback fires for any of the specified events
---@param events string[] Array of event names
---@param handler function(payload: table) Callback function
---@return table<string, integer> handler_ids Map of event names to handler IDs
local ids = niri.events:on({"window:open", "window:close", "window:focus"}, function(event)
    print("Window event:", event.id)
end)

-- One-time subscription (single event)
---@param event string Event name
---@param handler function(payload: table) Callback (called once, then removed)
---@return integer handler_id
niri.events:once("window:focus", function(event)
    print("First focus:", event.app_id)
end)

-- One-time subscription (multiple events)
---@param events string[] Array of event names
---@param handler function(payload: table) Each handler fires once independently
---@return table<string, integer> handler_ids
niri.events:once({"window:open", "window:close"}, function(event)
    print("First occurrence of either event")
end)

-- Unsubscribe (single event)
---@param event string Event name
---@param handler_id integer ID returned from :on() or :once()
---@return boolean success True if handler was found and removed
niri.events:off("window:open", id)

-- Unsubscribe (multiple events using table from :on()/:once())
---@param handler_ids table<string, integer> Handler ID map from multi-event subscription
---@return table<string, boolean> results Map of event names to removal success
niri.events:off(ids)  -- Remove all handlers registered with multi-event :on()

---@param event string Event name
---@param payload table Custom event data
---@return nil
niri.events:emit("custom_event", { data = "value" })

---@return string[] List of registered event names
local events = niri.events:list()

---@param event string Event name
---@return integer count Number of handlers removed
niri.events:clear("window:open")
```

### Event Payload Types

> **For LLM Agents**: Use these type definitions for event handler parameters.

```yaml
# Event Payload Type Definitions

WindowOpenPayload:
  type: object
  properties:
    id: { type: integer, description: "Unique window ID" }
    app_id: { type: string, nullable: true, description: "Wayland app_id (may be nil)" }
    title: { type: string, nullable: true, description: "Window title (may be nil)" }
    workspace_id: { type: integer, description: "Workspace the window opened on" }

WindowClosePayload:
  type: object
  properties:
    id: { type: integer }
    app_id: { type: string, nullable: true }
    title: { type: string, nullable: true }

WindowFocusPayload:
  type: object
  properties:
    id: { type: integer }
    app_id: { type: string, nullable: true }
    title: { type: string, nullable: true }

WindowBlurPayload:
  type: object
  properties:
    id: { type: integer }
    title: { type: string, nullable: true }

WorkspacePayload:
  type: object
  properties:
    name: { type: string, nullable: true, description: "Workspace name (nil if unnamed)" }
    idx: { type: integer, description: "1-based workspace index" }
    output: { type: string, description: "Output connector name" }

MonitorPayload:
  type: object
  properties:
    name: { type: string, description: "Human-readable name (make + model)" }
    connector: { type: string, description: "Connector name (e.g., eDP-1)" }

LayoutModePayload:
  type: object
  properties:
    floating_active: { type: boolean, description: "True if floating mode is active" }

LayoutWindowPayload:
  type: object
  properties:
    window_id: { type: integer, description: "Window ID affected" }

EmptyPayload:
  type: object
  properties: {}
```

### Available Events (14 total)

#### Window Events (4)

| Event | Payload Type | When Emitted |
|-------|--------------|--------------|
| `window:open` | `WindowOpenPayload` | Window is created and mapped |
| `window:close` | `WindowClosePayload` | Window is unmapped and destroyed |
| `window:focus` | `WindowFocusPayload` | Window gains keyboard focus |
| `window:blur` | `WindowBlurPayload` | Window loses keyboard focus |

```lua
-- Example: Track window lifecycle
niri.events:on("window:open", function(ev)
    ---@type { id: integer, app_id: string?, title: string?, workspace_id: integer }
    niri.utils.log(string.format("Window %d opened: %s (%s)",
        ev.id, ev.title or "untitled", ev.app_id or "unknown"))
end)

niri.events:on("window:close", function(ev)
    ---@type { id: integer, app_id: string?, title: string? }
    niri.utils.log(string.format("Window %d closed", ev.id))
end)
```

#### Workspace Events (2)

| Event | Payload Type | When Emitted |
|-------|--------------|--------------|
| `workspace:activate` | `WorkspacePayload` | Workspace becomes active (focused) |
| `workspace:deactivate` | `WorkspacePayload` | Workspace becomes inactive |

```lua
-- Example: Log workspace switches
niri.events:on("workspace:activate", function(ev)
    ---@type { name: string?, idx: integer, output: string }
    local name = ev.name or tostring(ev.idx)
    niri.utils.log(string.format("Switched to workspace %s on %s", name, ev.output))
end)
```

#### Monitor Events (2)

| Event | Payload Type | When Emitted |
|-------|--------------|--------------|
| `monitor:connect` | `MonitorPayload` | Output is connected and enabled |
| `monitor:disconnect` | `MonitorPayload` | Output is disconnected |

```lua
-- Example: React to monitor changes
niri.events:on("monitor:connect", function(ev)
    ---@type { name: string, connector: string }
    niri.utils.log(string.format("Monitor connected: %s (%s)", ev.name, ev.connector))
end)
```

#### Layout Events (3)

| Event | Payload Type | When Emitted |
|-------|--------------|--------------|
| `layout:mode_changed` | `LayoutModePayload` | Switch between tiling/floating mode |
| `layout:window_added` | `LayoutWindowPayload` | Window added to layout tree |
| `layout:window_removed` | `LayoutWindowPayload` | Window removed from layout tree |

```lua
-- Example: Track layout changes
niri.events:on("layout:mode_changed", function(ev)
    ---@type { floating_active: boolean }
    local mode = ev.floating_active and "floating" or "tiling"
    niri.utils.log("Layout mode: " .. mode)
end)
```

#### Config Events (1)

| Event | Payload Type | When Emitted |
|-------|--------------|--------------|
| `config:reload` | `EmptyPayload` | Configuration was reloaded |

```lua
-- Example: React to config reload
niri.events:on("config:reload", function(ev)
    ---@type {}
    niri.utils.log("Configuration reloaded")
end)
```

#### Overview Events (2)

| Event | Payload Type | When Emitted |
|-------|--------------|--------------|
| `overview:open` | `EmptyPayload` | Overview mode opened |
| `overview:close` | `EmptyPayload` | Overview mode closed |

```lua
-- Example: Track overview state
niri.events:on("overview:open", function(ev)
    ---@type {}
    niri.utils.log("Overview opened")
end)
```

### Intentionally Unimplemented Events

The following events are **intentionally not wired** for security reasons:

#### Idle Events (`idle:start`, `idle:end`)

**Not implemented.** Exposing idle state to Lua scripts creates security risks:

- **Presence detection**: Malicious scripts could track when users are away from their computer
- **Targeted attacks**: Scripts could wait for idle state to perform unwanted actions undetected
- **Privacy violation**: User activity patterns could be logged or exfiltrated

Unlike AwesomeWM (which runs in a trusted X11 environment where the user controls all code), niri's Lua environment may eventually support third-party plugins. The idle inhibitor protocol provides the sanctioned way for applications to prevent idle.

#### Key Events (`key:press`, `key:release`)

**Not implemented.** Exposing raw key events to Lua scripts is a severe security risk:

- **Keylogging**: Scripts could capture passwords, private messages, and sensitive data
- **Credential theft**: Banking credentials, API keys, and authentication tokens could be stolen
- **Privacy violation**: Complete record of user input could be exfiltrated

AwesomeWM exposes key events because it operates in a single-user X11 model where the window manager configuration is fully trusted. Niri takes a defense-in-depth approach: keybindings are configured declaratively in the config, and Lua scripts receive only high-level events (window focus, workspace changes) that don't leak sensitive input.

**Alternative (TODO - not yet implemented)**: A future `lua-action` binding type would allow triggering named Lua functions from keybindings:
```kdl
binds {
    Mod+X { lua-action "my_custom_action"; }
}
```
This would call a registered Lua function without exposing raw key events. See [Future: Custom Keybinding Actions](#future-custom-keybinding-actions) for the planned design.

### Event Handler Safety

Event handlers execute with timeout protection (default 1 second). Long-running handlers will be interrupted:

```lua
niri.events:on("window:open", function(event)
    -- BAD: This will timeout
    while true do end

    -- GOOD: Keep handlers fast
    print(event.app_id)
end)
```

---

## Action System

The `niri.action` namespace provides access to all niri actions. Actions are first-class values that can be stored, passed around, and executed.

> **For LLM Agents**: This section documents ~90 actions with full type signatures and error behavior.

### Basic Usage

```lua
-- Execute immediately
niri.action.close_window()
niri.action.spawn({"alacritty"})

-- Store for later
local my_action = niri.action.focus_workspace_down()

-- Use in bindings
niri.config.binds:add("Mod+Q", niri.action.close_window())
```

### Action Signatures (~90 actions)

> **Signature Format**: `action_name(params) -> return_type | ErrorBehavior`

#### Application Actions

| Action | Signature | Description | Error Behavior |
|--------|-----------|-------------|----------------|
| `quit` | `() -> nil` | Exit niri with confirmation dialog | N/A |
| `quit_skip_confirmation` | `() -> nil` | Exit niri immediately | N/A |
| `spawn` | `(cmd: string[]) -> nil` | Spawn process | Logs error if spawn fails |
| `spawn_at_startup` | `(cmd: string[]) -> nil` | Register for startup spawn | N/A |
| `spawn_sh` | `(cmd: string) -> nil` | Spawn via `$SHELL -c` | Logs error if spawn fails |

```lua
---@param cmd string[] Command and arguments array
---@return nil
---@error Logs to stderr if spawn fails; does not throw
niri.action.spawn({"alacritty", "--working-directory", "/tmp"})

---@param cmd string Shell command string
---@return nil
niri.action.spawn_sh("notify-send 'Hello World'")
```

#### Window Actions

| Action | Signature | Description | Error Behavior |
|--------|-----------|-------------|----------------|
| `close_window` | `() -> nil` | Close focused window | No-op if no window focused |
| `focus_window` | `(id: integer) -> boolean` | Focus window by ID | Returns false if not found |
| `focus_window_up` | `() -> nil` | Focus window above | No-op at boundary |
| `focus_window_down` | `() -> nil` | Focus window below | No-op at boundary |
| `focus_window_left` | `() -> nil` | Focus window left | No-op at boundary |
| `focus_window_right` | `() -> nil` | Focus window right | No-op at boundary |
| `focus_window_up_or_column_left` | `() -> nil` | Focus up, wrap to column left | Wraps at boundary |
| `focus_window_down_or_column_right` | `() -> nil` | Focus down, wrap to column right | Wraps at boundary |
| `move_window_up` | `() -> nil` | Move window up in column | No-op at boundary |
| `move_window_down` | `() -> nil` | Move window down in column | No-op at boundary |
| `move_window_up_or_to_workspace_up` | `() -> nil` | Move up, wrap to workspace | Creates workspace if needed |
| `move_window_down_or_to_workspace_down` | `() -> nil` | Move down, wrap to workspace | Creates workspace if needed |
| `consume_or_expel_window_left` | `() -> nil` | Consume from left or expel left | Context-dependent |
| `consume_or_expel_window_right` | `() -> nil` | Consume from right or expel right | Context-dependent |
| `consume_window_into_column` | `() -> nil` | Consume adjacent window | No-op if no adjacent |
| `expel_window_from_column` | `() -> nil` | Expel window to new column | No-op if single window |
| `fullscreen_window` | `() -> nil` | Toggle fullscreen | No-op if no window |
| `toggle_window_floating` | `() -> nil` | Toggle floating state | No-op if no window |
| `toggle_window_urgent` | `() -> nil` | Toggle urgency (Since 25.11) | No-op if no window |
| `focus_urgent_or_previous_window` | `() -> nil` | Focus urgent or MRU (Since 25.11) | No-op if none available |

```lua
---@param id integer Window ID from niri.state.windows
---@return boolean success True if window was found and focused
niri.action.focus_window(id)
```

#### Column Actions

| Action | Signature | Description | Error Behavior |
|--------|-----------|-------------|----------------|
| `focus_column_left` | `() -> nil` | Focus column to left | No-op at boundary |
| `focus_column_right` | `() -> nil` | Focus column to right | No-op at boundary |
| `focus_column_first` | `() -> nil` | Focus first column | No-op if no columns |
| `focus_column_last` | `() -> nil` | Focus last column | No-op if no columns |
| `focus_column_right_or_first` | `() -> nil` | Focus right, wrap to first | Wraps |
| `focus_column_left_or_last` | `() -> nil` | Focus left, wrap to last | Wraps |
| `move_column_left` | `() -> nil` | Move column left | No-op at boundary |
| `move_column_right` | `() -> nil` | Move column right | No-op at boundary |
| `move_column_left_or_to_monitor_left` | `() -> nil` | Move left, wrap to monitor | No-op if no monitor |
| `move_column_right_or_to_monitor_right` | `() -> nil` | Move right, wrap to monitor | No-op if no monitor |
| `move_column_to_first` | `() -> nil` | Move column to first position | No-op if already first |
| `move_column_to_last` | `() -> nil` | Move column to last position | No-op if already last |
| `center_column` | `() -> nil` | Center column in view | No-op if no column |
| `toggle_column_tabbed` | `() -> nil` | Toggle tabbed display | No-op if single window |
| `toggle_column_tabbed_display` | `() -> nil` | Toggle tabbed (Since 25.11) | No-op if single window |
| `expand_column_to_available_width` | `() -> nil` | Expand to fill (Since 25.05) | No-op if already full |
| `center_visible_columns` | `() -> nil` | Center all visible (Since 25.11) | No-op if no columns |

#### Workspace Actions

| Action | Signature | Description | Error Behavior |
|--------|-----------|-------------|----------------|
| `focus_workspace_up` | `() -> nil` | Focus workspace above | No-op at first |
| `focus_workspace_down` | `() -> nil` | Focus workspace below | Creates if needed |
| `focus_workspace` | `(ref: integer\|string) -> nil` | Focus by index or name | Creates if name not found |
| `focus_workspace_previous` | `() -> nil` | Focus previously active | No-op if no previous |
| `move_window_to_workspace_up` | `() -> nil` | Move window up | Creates workspace if needed |
| `move_window_to_workspace_down` | `() -> nil` | Move window down | Creates workspace if needed |
| `move_window_to_workspace` | `(ref: integer\|string) -> nil` | Move to index/name | Creates if needed |
| `move_column_to_workspace_up` | `() -> nil` | Move column up | Creates if needed |
| `move_column_to_workspace_down` | `() -> nil` | Move column down | Creates if needed |
| `move_column_to_workspace` | `(ref: integer\|string) -> nil` | Move column to workspace | Creates if needed |
| `move_workspace_up` | `() -> nil` | Reorder workspace up | No-op at first |
| `move_workspace_down` | `() -> nil` | Reorder workspace down | No-op at last |
| `move_workspace_to_index` | `(idx: integer) -> nil` | Move to specific index | Clamped to valid range |

```lua
---@param ref integer|string Workspace index (1-based) or name
---@return nil
---@note Creates named workspace if name doesn't exist
niri.action.focus_workspace(1)        -- By index
niri.action.focus_workspace("browser") -- By name
```

#### Monitor Actions

| Action | Signature | Description | Error Behavior |
|--------|-----------|-------------|----------------|
| `focus_monitor_left` | `() -> nil` | Focus monitor to left | No-op if none |
| `focus_monitor_right` | `() -> nil` | Focus monitor to right | No-op if none |
| `focus_monitor_up` | `() -> nil` | Focus monitor above | No-op if none |
| `focus_monitor_down` | `() -> nil` | Focus monitor below | No-op if none |
| `focus_monitor` | `(name: string) -> nil` | Focus by output name | No-op if not found |
| `focus_monitor_next` | `() -> nil` | Focus next monitor | Wraps around |
| `focus_monitor_previous` | `() -> nil` | Focus previous monitor | Wraps around |
| `move_window_to_monitor_left` | `() -> nil` | Move window left | No-op if no monitor |
| `move_window_to_monitor_right` | `() -> nil` | Move window right | No-op if no monitor |
| `move_window_to_monitor` | `(name: string) -> nil` | Move to named output | No-op if not found |
| `move_column_to_monitor_left` | `() -> nil` | Move column left | No-op if no monitor |
| `move_column_to_monitor_right` | `() -> nil` | Move column right | No-op if no monitor |
| `move_column_to_monitor` | `(name: string) -> nil` | Move to named output | No-op if not found |
| `move_workspace_to_monitor_left` | `() -> nil` | Move workspace left | No-op if no monitor |
| `move_workspace_to_monitor_right` | `() -> nil` | Move workspace right | No-op if no monitor |
| `move_workspace_to_monitor` | `(name: string) -> nil` | Move to named output | No-op if not found |

```lua
---@param name string Output connector name (e.g., "eDP-1", "HDMI-A-1")
---@return nil
---@note Silently ignored if output not connected
niri.action.focus_monitor("eDP-1")
```

#### Size Actions

| Action | Signature | Description | Error Behavior |
|--------|-----------|-------------|----------------|
| `set_column_width` | `(spec: SizeSpec\|string) -> nil` | Set column width | Clamped to valid range |
| `set_window_height` | `(spec: SizeSpec\|string) -> nil` | Set window height | Clamped to valid range |
| `switch_preset_column_width` | `() -> nil` | Cycle width presets | Wraps around |
| `switch_preset_window_height` | `() -> nil` | Cycle height presets | Wraps around |
| `reset_window_height` | `() -> nil` | Reset to automatic height | N/A |
| `maximize_column` | `() -> nil` | Maximize column width | N/A |

```lua
---@alias SizeSpec { proportion: number } | { fixed: integer }
---@param spec SizeSpec|string Size specification
---  - { proportion = 0.5 } -- 50% of workspace
---  - { fixed = 800 }      -- 800 pixels
---  - "+10%"               -- Increase by 10%
---  - "-100"               -- Decrease by 100px
---@return nil
niri.action.set_column_width({ proportion = 0.5 })
niri.action.set_column_width({ fixed = 800 })
niri.action.set_column_width("+10%")
niri.action.set_column_width("-100")
```

#### Screenshot Actions

| Action | Signature | Description | Error Behavior |
|--------|-----------|-------------|----------------|
| `screenshot` | `() -> nil` | Screenshot all outputs | Saves to screenshot_path |
| `screenshot_screen` | `() -> nil` | Screenshot current output | Saves to screenshot_path |
| `screenshot_window` | `() -> nil` | Screenshot focused window | No-op if no window |
| `confirm_screenshot` | `() -> nil` | Confirm interactive selection | No-op if not in screenshot mode |
| `cancel_screenshot` | `() -> nil` | Cancel interactive selection | No-op if not in screenshot mode |

#### Output/Power Actions

| Action | Signature | Description | Error Behavior |
|--------|-----------|-------------|----------------|
| `power_off_monitors` | `() -> nil` | Turn off all monitors | N/A |
| `power_on_monitors` | `() -> nil` | Turn on all monitors | N/A |
| `set_dynamic_cast_window` | `(id: integer) -> nil` | Set cast target (Since 25.11) | No-op if not found |
| `set_dynamic_cast_monitor` | `(name: string) -> nil` | Set cast target (Since 25.11) | No-op if not found |
| `clear_dynamic_cast_target` | `() -> nil` | Clear cast target (Since 25.11) | N/A |

#### Keyboard/Input Actions

| Action | Signature | Description | Error Behavior |
|--------|-----------|-------------|----------------|
| `toggle_keyboard_shortcuts_inhibit` | `() -> nil` | Toggle inhibit (Since 25.05) | N/A |
| `set_keyboard_shortcuts_inhibit` | `(enable: boolean) -> nil` | Set inhibit state | N/A |
| `switch_layout` | `(dir: "next"\|"prev") -> nil` | Switch keyboard layout | Wraps around |

#### UI Actions

| Action | Signature | Description | Error Behavior |
|--------|-----------|-------------|----------------|
| `toggle_overview` | `() -> nil` | Toggle overview mode | N/A |
| `show_hotkey_overlay` | `() -> nil` | Show hotkey overlay | N/A |
| `do_screen_transition` | `(delay_ms?: integer) -> nil` | Screen transition | Default 0ms |

#### Debug Actions

| Action | Signature | Description | Error Behavior |
|--------|-----------|-------------|----------------|
| `toggle_debug_tint` | `() -> nil` | Toggle debug tint | N/A |
| `debug_toggle_opaque_regions` | `() -> nil` | Toggle opaque region debug | N/A |
| `debug_toggle_damage` | `() -> nil` | Toggle damage debug | N/A |

#### Special Actions

| Action | Signature | Description | Error Behavior |
|--------|-----------|-------------|----------------|
| `noop` | `() -> nil` | No operation | N/A |

---

## Timer API

The `niri.loop` namespace provides timer functionality for deferred and repeating callbacks.

### Creating Timers

```lua
-- Create a timer
local timer = niri.loop.new_timer()

-- One-shot timer (fires once after delay)
timer:start(1000, 0, function()
    print("Fired after 1 second")
end)

-- Repeating timer (fires every interval)
timer:start(0, 500, function()
    print("Fires every 500ms")
end)

-- Delayed repeating timer
timer:start(1000, 500, function()
    print("First after 1s, then every 500ms")
end)
```

### Timer Methods

```lua
-- Start timer: (delay_ms, repeat_ms, callback)
timer:start(delay, repeat_interval, callback)

-- Stop timer (can be restarted)
timer:stop()

-- Restart with same settings
timer:again()

-- Check if timer is active
local active = timer:is_active()

-- Close timer permanently (cleanup)
timer:close()
```

### Get Current Time

```lua
-- Get current time in milliseconds (since compositor start)
local now = niri.loop.now()
```

### Timer Lifetime

Timers persist until explicitly closed, even if the Lua handle is garbage collected:

```lua
-- Timer continues even without handle reference (BAD - memory leak)
niri.loop.new_timer():start(1000, 0, function()
    print("Still fires!")  -- GC doesn't stop this
end)

-- Proper cleanup pattern (GOOD)
local timer = niri.loop.new_timer()
timer:start(1000, 0, function()
    -- Do work
    timer:close()  -- Explicit cleanup
end)
```

### Timer Examples

```lua
-- Auto-save workspace layout every 5 minutes
local autosave = niri.loop.new_timer()
autosave:start(0, 300000, function()
    -- Save workspace layout
end)

-- Debounce rapid events
local debounce_timer = niri.loop.new_timer()
local pending_update = nil

niri.events:on("window:open", function(event)
    pending_update = event
    debounce_timer:stop()
    debounce_timer:start(100, 0, function()
        if pending_update then
            -- Process after 100ms of no new events
            handle_update(pending_update)
            pending_update = nil
        end
    end)
end)
```

---

## Async and Scheduling

### Deferred Execution

`niri.schedule()` queues callbacks for execution after the current event loop cycle:

```lua
niri.schedule(function()
    -- Runs after current operation completes
    print("Deferred execution")
end)
```

**Limits (module-level constants):**
- Maximum 16 callbacks processed per cycle (`MAX_CALLBACKS_PER_FLUSH`)
- Maximum 1000 callbacks in queue (`MAX_QUEUE_SIZE`)
- Exceeding limits drops oldest callbacks

### Timeout Protection

All Lua execution is protected by configurable timeouts:

```lua
-- Default: 1 second timeout
-- Long-running code will be interrupted:

niri.events:on("window:open", function(event)
    -- This will timeout and be interrupted:
    for i = 1, 1e12 do end
end)
```

The Luau dialect's `set_interrupt` mechanism enables reliable interruption without undefined behavior.

### Processing Cycle

Each frame, `process_async()` handles:
1. Scheduled callbacks (up to `MAX_CALLBACKS_PER_FLUSH`)
2. Timer callbacks (expired timers)

---

## Utility API

> **Status**: NOT IMPLEMENTED - All functions below are planned but not yet available.

The `niri.utils` namespace will provide OS-level utility functions. Since niri-lua uses Luau's safe mode (all standard `os.*` and `io.*` functions are blocked), these utilities will expose essential functionality through the Rust host.

This follows the **Neovim pattern** where `vim.fn.*` provides system utilities that the underlying Lua runtime cannot access directly.

### Planned Functions

| Function | Status | Equivalent |
|----------|--------|------------|
| `getenv(name)` | Planned | `vim.env.NAME` |
| `stdpath(what)` | Planned | `vim.fn.stdpath()` |
| `hostname` | Planned | `vim.fn.hostname()` |
| `executable(name)` | Planned | `vim.fn.executable()` |
| `file_readable(path)` | Planned | `vim.fn.filereadable()` |
| `is_directory(path)` | Planned | `vim.fn.isdirectory()` |
| `expand(path)` | Planned | `vim.fn.expand()` |
| `read_file(path)` | Future | `vim.fn.readfile()` |
| `glob(pattern)` | Future | `vim.fn.glob()` |

### Example Use Cases (Once Implemented)

**Conditional Configuration:**
```lua
-- Host-specific config
if niri.utils.hostname == "laptop" then
    niri.config.input.touchpad.tap = true
end

-- Tool-dependent config
if niri.utils.executable("swaylock") then
    -- Use swaylock for locking
elseif niri.utils.executable("hyprlock") then
    -- Fall back to hyprlock
end
```

**Plugin Bootstrapping:**
```lua
-- Check if plugin directory exists
local plugin_dir = niri.utils.stdpath("data") .. "/plugins"
if not niri.utils.is_directory(plugin_dir) then
    -- First run: create directory
end
```

---

## Validation Rules

> **For LLM Agents**: Use these rules to validate generated configurations before execution.

### Color Validation

```lua
-- Valid color formats
local valid_colors = {
    "#fff",        -- 3-digit hex (RGB)
    "#ffff",       -- 4-digit hex (RGBA)
    "#ffffff",     -- 6-digit hex (RRGGBB)
    "#ffffff80",   -- 8-digit hex (RRGGBBAA)
}

-- Invalid formats (will cause errors)
local invalid_colors = {
    "red",             -- Named colors NOT supported
    "rgb(255,0,0)",    -- CSS function NOT supported
    "#gg0000",         -- Invalid hex characters
    "ffffff",          -- Missing # prefix
}

-- Validation regex
local color_pattern = "^#([0-9a-fA-F]{3}|[0-9a-fA-F]{4}|[0-9a-fA-F]{6}|[0-9a-fA-F]{8})$"
```

### Numeric Ranges

| Field | Type | Range | Behavior Outside Range |
|-------|------|-------|------------------------|
| `layout.gaps` | integer | [0, ∞) | Clamped to 0 |
| `focus_ring.width` | integer | [0, 100] | Clamped |
| `accel_speed` | float | [-1.0, 1.0] | Clamped |
| `opacity` | float | [0.0, 1.0] | Clamped |
| `proportion` | float | (0.0, 1.0] | Clamped, 0 becomes minimum |
| `angle` | float | [0, 360) | Modulo 360 |
| `repeat_delay` | integer | [1, 10000] | Clamped |
| `repeat_rate` | integer | [1, 1000] | Clamped |
| `scale` | float | [0.25, 10.0] | Clamped |
| `slowdown` | float | [0.0, 100.0] | Clamped |
| `spring.stiffness` | float | (0, 10000] | Clamped |
| `spring.damping_ratio` | float | [0, 10] | Clamped |

### String Validation

```lua
-- Output/monitor names: Match connector names exactly
local valid_output_names = { "eDP-1", "HDMI-A-1", "DP-1", "DP-2" }

-- Workspace names: Any non-empty string
local valid_workspace_names = { "browser", "terminal", "1", "main" }

-- Key patterns: Must match modifier+key format
local valid_keys = {
    "Mod+Q",
    "Mod+Shift+Return",
    "Ctrl+Alt+Delete",
    "XF86AudioRaiseVolume",
    "Mod+MouseLeft",
    "Mod+WheelScrollDown",
}

-- Screenshot path: Supports strftime placeholders
local valid_screenshot_path = "~/Pictures/Screenshots/%Y-%m-%d_%H-%M-%S.png"
-- Placeholders: %Y (year), %m (month), %d (day), %H (hour), %M (minute), %S (second)
```

### Enum Validation

```lua
-- All enum values are case-sensitive strings
local enums = {
    center_focused_column = { "never", "always", "on-overflow" },
    accel_profile = { "adaptive", "flat" },
    click_method = { "button-areas", "clickfinger" },
    scroll_method = { "two-finger", "edge", "on-button-down" },
    track_layout = { "global", "window" },
    transform = { "normal", "90", "180", "270", "flipped", "flipped-90", "flipped-180", "flipped-270" },
    relative_to = { "workspace-view", "window" },
    block_out_from = { "screencast", "screen-capture" },
    column_display = { "normal", "tabbed" },
    tab_indicator_position = { "left", "right", "top", "bottom" },
    floating_position_relative = { "top-left", "top-right", "bottom-left", "bottom-right", "center" },
}
```

### Collection Constraints

```lua
-- Window rules: matches array must have at least one match
niri.config.window_rules:add({
    matches = {},  -- ERROR: Empty matches not allowed
    open_floating = true,
})

-- Bindings: Key must be unique (later binds override earlier)
niri.config.binds:add("Mod+Q", action1)
niri.config.binds:add("Mod+Q", action2)  -- OK: Overrides previous

-- Outputs: name is required
niri.config.outputs:add({
    -- ERROR: 'name' is required
    scale = 2.0,
})
```

---

## Error Reference

> **For LLM Agents**: Common errors and their resolutions.

### Configuration Errors

| Error | Cause | Resolution |
|-------|-------|------------|
| `invalid color format` | Color string doesn't match hex pattern | Use `#RGB`, `#RGBA`, `#RRGGBB`, or `#RRGGBBAA` |
| `unknown field` | Table contains unrecognized key | Check spelling; refer to Schema Reference |
| `expected table, got X` | Wrong value type | Check expected type in Schema Reference |
| `invalid enum value` | String not in allowed values | Use exact string from enum list |
| `missing required field` | Required field not provided | Add the required field |
| `regex parse error` | Invalid regex pattern | Fix regex syntax (uses Rust regex crate) |

### Runtime Errors

| Error | Cause | Resolution |
|-------|-------|------------|
| `window not found` | Window ID doesn't exist | Query `niri.state.windows` for valid IDs |
| `output not found` | Output name not connected | Query `niri.state.outputs` for valid names |
| `workspace not found` | Workspace doesn't exist | Will be created for named workspaces |
| `action execution failed` | Action couldn't complete | Check preconditions (e.g., window focused) |
| `script timeout` | Script exceeded execution limit | Optimize script; avoid infinite loops |
| `memory limit exceeded` | Script used too much memory | Reduce allocations; avoid large tables |

### Binding Errors

| Error | Cause | Resolution |
|-------|-------|------------|
| `invalid key` | Key string not recognized | Check key name spelling |
| `invalid modifier` | Modifier not recognized | Use: `Mod`, `Ctrl`, `Shift`, `Alt`, `Super` |
| `action required` | Binding without action | Provide action or use `niri.action.noop()` |

### Error Handling Pattern

```lua
-- Defensive configuration pattern
local function safe_add_output(name, config)
    -- Check if output exists before configuring
    local outputs = niri.state.outputs
    local found = false
    for _, o in ipairs(outputs) do
        if o.name == name then
            found = true
            break
        end
    end

    if found then
        config.name = name
        niri.config.outputs:add(config)
        niri.utils.log("Configured output: " .. name)
    else
        niri.utils.warn("Output not found, config will apply when connected: " .. name)
        config.name = name
        niri.config.outputs:add(config)  -- Still add; will apply when connected
    end
end
```

---

## Cookbook

> **For LLM Agents**: Tested recipes for common tasks. Copy and adapt these patterns.

### Recipe 1: Auto-Float Dialog Windows

```lua
-- Float all windows that look like dialogs
niri.config.window_rules:add({
    matches = {
        { title = ".*[Dd]ialog.*", is_regex = true },
        { title = ".*[Pp]references.*", is_regex = true },
        { title = ".*[Ss]ettings.*", is_regex = true },
        { app_id = ".*-dialog$", is_regex = true },
    },
    open_floating = true,
})
```

### Recipe 2: Application-Specific Workspace Assignment

```lua
-- Define named workspaces
niri.config.workspaces:add({ name = "browser", open_on_output = "eDP-1" })
niri.config.workspaces:add({ name = "code", open_on_output = "eDP-1" })
niri.config.workspaces:add({ name = "chat", open_on_output = "HDMI-A-1" })

-- Assign apps to workspaces
niri.config.window_rules:add({
    matches = { { app_id = "firefox" }, { app_id = "chromium" } },
    open_on_workspace = "browser",
})

niri.config.window_rules:add({
    matches = { { app_id = "code" }, { app_id = "Code" } },
    open_on_workspace = "code",
})

niri.config.window_rules:add({
    matches = { { app_id = "discord" }, { app_id = "slack" }, { app_id = "telegram" } },
    open_on_workspace = "chat",
})
```

### Recipe 3: Dynamic Window Focus Logging

```lua
-- Log all window focus changes (useful for debugging)
niri.events:on("window:focus", function(ev)
    local timestamp = os.date("%H:%M:%S")
    niri.utils.log(string.format("[%s] Focus: %s (%s) id=%d",
        timestamp,
        ev.title or "untitled",
        ev.app_id or "unknown",
        ev.id
    ))
end)
```

### Recipe 4: Workspace Indicator in Status Bar

```lua
-- Track active workspace for external status bar
local current_workspace = { name = nil, idx = 1, output = nil }

niri.events:on("workspace:activate", function(ev)
    current_workspace = ev
    -- Write to file for polybar/waybar/etc.
    local f = io.open("/tmp/niri-workspace", "w")
    if f then
        f:write(ev.name or tostring(ev.idx))
        f:close()
    end
end)
```

### Recipe 5: Picture-in-Picture Window Rules

```lua
-- Firefox/Chrome PiP windows: Float in corner, always on top
niri.config.window_rules:add({
    matches = {
        { title = "Picture-in-Picture" },
        { title = "Picture in picture" },
    },
    open_floating = true,
    default_floating_position = { x = 20, y = 20, relative_to = "bottom-right" },
    opacity = 1.0,
})
```

### Recipe 6: Emergency Window Recovery

```lua
-- Keybind to force-focus first window (recovery from stuck state)
niri.config.binds:add("Mod+Shift+Escape", (function()
    local windows = niri.state.windows
    if #windows > 0 then
        niri.action.focus_window(windows[1].id)
    end
end)())
```

### Recipe 7: Output-Aware Configuration

```lua
-- Apply different layouts based on connected outputs
niri.events:on("monitor:connect", function(ev)
    if ev.connector == "HDMI-A-1" then
        -- External monitor connected: adjust layout
        niri.config.layout.gaps = 24
        niri.config.layout.focus_ring.width = 6
        niri.config:apply()
        niri.utils.log("Applied external monitor layout")
    end
end)

niri.events:on("monitor:disconnect", function(ev)
    if ev.connector == "HDMI-A-1" then
        -- External disconnected: restore laptop layout
        niri.config.layout.gaps = 8
        niri.config.layout.focus_ring.width = 2
        niri.config:apply()
        niri.utils.log("Applied laptop layout")
    end
end)
```

### Recipe 8: Keybinding Groups with Cooldown

```lua
-- Media controls with cooldown to prevent accidental double-triggers
local media_binds = {
    { key = "XF86AudioPlay", cmd = {"playerctl", "play-pause"} },
    { key = "XF86AudioNext", cmd = {"playerctl", "next"} },
    { key = "XF86AudioPrev", cmd = {"playerctl", "previous"} },
    { key = "XF86AudioRaiseVolume", cmd = {"wpctl", "set-volume", "@DEFAULT_AUDIO_SINK@", "5%+"} },
    { key = "XF86AudioLowerVolume", cmd = {"wpctl", "set-volume", "@DEFAULT_AUDIO_SINK@", "5%-"} },
    { key = "XF86AudioMute", cmd = {"wpctl", "set-mute", "@DEFAULT_AUDIO_SINK@", "toggle"} },
}

for _, bind in ipairs(media_binds) do
    niri.config.binds:add(bind.key, {
        action = niri.action.spawn(bind.cmd),
        allow_when_locked = true,
        cooldown_ms = 100,
    })
end
```

### Recipe 9: Window Opacity by Application

```lua
-- Adjust opacity for specific apps
local opacity_rules = {
    { app_id = "Alacritty", opacity = 0.92 },
    { app_id = "kitty", opacity = 0.92 },
    { app_id = "code", opacity = 0.98 },
    { app_id = "discord", opacity = 0.95 },
}

for _, rule in ipairs(opacity_rules) do
    niri.config.window_rules:add({
        matches = { { app_id = rule.app_id } },
        opacity = rule.opacity,
    })
end
```

### Recipe 10: Startup Applications

```lua
-- Applications to launch at startup
local startup_apps = {
    {"waybar"},
    {"mako"},
    {"nm-applet"},
    {"/usr/lib/polkit-gnome/polkit-gnome-authentication-agent-1"},
}

for _, cmd in ipairs(startup_apps) do
    niri.action.spawn_at_startup(cmd)
end

-- Alternative: Use spawn_at_startup array
niri.config.spawn_at_startup = startup_apps
```

---

## REPL

Niri includes an interactive Lua REPL for debugging and exploration.

### Starting the REPL

```bash
# Start REPL (requires running niri instance)
niri msg repl
```

### REPL Features

- Full access to `niri.*` APIs
- Multi-line input support
- History navigation
- Tab completion (planned)

### Example Session

```
niri> local wins = niri.state.windows()
niri> for _, w in ipairs(wins) do print(w.app_id, w.title) end
firefox Mozilla Firefox
alacritty Alacritty
niri> niri.action.spawn({"wofi", "--show", "drun"})
niri> niri.events:list()
{"window:open", "window:close", "window:focus", ...}
```

### REPL Commands

```
niri> :help      -- Show help
niri> :quit      -- Exit REPL
niri> :clear     -- Clear screen
```

---

## Plugin System

niri-lua follows the **Neovim model** for plugins: plugins are pure Lua modules loaded explicitly via `require()`. There is no auto-discovery or auto-loading; users control exactly what code runs in their config.

### Plugin Locations

Plugins are Lua modules placed in the user's config directory:

```
~/.config/niri/lua/       -- User plugins (added to package.path)
~/.config/niri/plugins/   -- Alternative convention
```

The custom module loader adds these directories to `package.path`, allowing standard `require()` to find them.

### Plugin Structure

Plugins follow the Neovim convention: a module table with a `setup()` function.

```lua
-- ~/.config/niri/lua/my-plugin.lua

local M = {}

M.config = {
    -- Default options
    enabled = true,
}

function M.setup(opts)
    M.config = vim.tbl_deep_extend("force", M.config, opts or {})

    -- Register event handlers
    niri.events:on("window:open", function(event)
        if M.config.enabled then
            -- Plugin logic
        end
    end)

    return M
end

return M
```

### Loading Plugins

Users explicitly load plugins in their `niri.lua`:

```lua
-- In niri.lua config
local my_plugin = require("my-plugin")
my_plugin.setup({ enabled = true })

-- Or inline for simple cases:
require("auto-float").setup()
```

This explicit loading model ensures:
- **Security**: No code runs without user consent
- **Clarity**: Config shows exactly what's loaded
- **Control**: Load order is deterministic

### Plugin Best Practices

1. **Keep handlers fast**: Respect timeout limits
2. **Clean up on unload**: Unsubscribe from events in a `M.disable()` function
3. **Namespace globals**: Use local modules, never pollute `_G`
4. **Document configuration**: Provide clear options with sensible defaults
5. **Fail gracefully**: Check for optional dependencies with `pcall(require, ...)`

### Future: niri.pack

A future `niri.pack` API (inspired by Neovim's `vim.pack`) may provide:
- Git-based plugin installation: `niri.pack.add("author/plugin")`
- Lockfile for reproducible configs
- Lazy loading support

This will remain opt-in; manual `require()` will always work.

---

## Type Definitions

EmmyLua type definitions are provided in `niri-lua/types/api.lua` for IDE support.

### Usage with Lua LSP

```lua
-- Add to your LSP configuration
-- types/api.lua provides annotations for niri.* APIs
```

### Key Types

```lua
---@class Window
---@field id number Unique window ID
---@field app_id string Application identifier
---@field title string Window title
---@field workspace_id number Containing workspace ID
---@field is_focused boolean Whether window has focus
---@field is_floating boolean Whether window is floating

---@class Workspace
---@field id number Unique workspace ID
---@field idx number Index on output (1-based)
---@field name string? Optional workspace name
---@field output string Output name
---@field is_active boolean Whether workspace is active on its output
---@field is_focused boolean Whether workspace has global focus
---@field active_window_id number? ID of active window, if any

---@class Output
---@field name string Output name (e.g., "eDP-1")
---@field make string Monitor manufacturer
---@field model string Monitor model
---@field serial string Monitor serial number
---@field width number Logical width in pixels
---@field height number Logical height in pixels
---@field refresh number Refresh rate in Hz
---@field scale number Scale factor
---@field transform string Applied transform
---@field is_enabled boolean Whether output is enabled

---@alias SizeChange { proportion: number } | { fixed: number } | string

---@class Timer
---@field start fun(self: Timer, delay_ms: number, repeat_ms: number, callback: fun())
---@field stop fun(self: Timer)
---@field again fun(self: Timer)
---@field is_active fun(self: Timer): boolean
---@field close fun(self: Timer)
```

---

## Examples

### Auto-move Windows by App

```lua
-- Move specific apps to designated workspaces
niri.events:on("window:open", function(event)
    if event.app_id == "slack" then
        niri.action.move_window_to_workspace("chat")
    elseif event.app_id == "spotify" then
        niri.action.move_window_to_workspace("media")
    end
end)
```

### Workspace Indicator

```lua
-- Print workspace changes
niri.events:on("workspace:activate", function(event)
    local name = event.name or ("Workspace " .. event.idx)
    print("Switched to: " .. name)
end)
```

### Window Counter

```lua
-- Track window count
local window_count = 0

niri.events:on("window:open", function()
    window_count = window_count + 1
    print("Windows: " .. window_count)
end)

niri.events:on("window:close", function()
    window_count = window_count - 1
    print("Windows: " .. window_count)
end)
```

### Dynamic Gaps

```lua
-- Adjust gaps based on window count
local function update_gaps()
    local windows = niri.state.windows()
    local count = #windows

    if count <= 1 then
        niri.config.layout.gaps = 0
    elseif count <= 4 then
        niri.config.layout.gaps = 8
    else
        niri.config.layout.gaps = 16
    end
end

niri.events:on("window:open", update_gaps)
niri.events:on("window:close", update_gaps)
```

### Scratchpad

```lua
-- Toggle a scratchpad terminal
local scratchpad_id = nil

local function toggle_scratchpad()
    if scratchpad_id then
        -- Find and focus existing
        local windows = niri.state.windows()
        for _, win in ipairs(windows) do
            if win.id == scratchpad_id then
                if win.is_focused then
                    -- Hide (move to special workspace)
                    niri.action.move_window_to_workspace("scratchpad")
                else
                    -- Show
                    niri.action.focus_window_by_id(scratchpad_id)
                end
                return
            end
        end
        -- Window was closed
        scratchpad_id = nil
    end

    -- Spawn new scratchpad
    niri.action.spawn({"alacritty", "--class", "scratchpad"})
end

niri.events:on("window:open", function(event)
    if event.app_id == "scratchpad" then
        scratchpad_id = event.id
        niri.action.toggle_window_floating()
    end
end)

niri.config.binds:add("Mod+grave", toggle_scratchpad)
```

---

## Future: Custom Keybinding Actions

> **Status**: TODO - Not yet implemented

### Motivation

Users need a way to trigger custom Lua functions from keybindings without exposing raw key events (which would be a security risk). This feature bridges the gap between the declarative KDL config and the Lua scripting system.

### Proposed Design

#### 1. Register Named Actions in Lua

```lua
-- In niri.lua or a plugin
niri.actions:register("my_custom_action", function()
    local focused = niri.state:focused_window()
    if focused and focused.app_id == "firefox" then
        niri.action.move_window_to_workspace(2)
    end
end)

niri.actions:register("toggle_my_layout", function()
    -- Custom layout toggle logic
end)
```

#### 2. Bind in KDL Config

```kdl
binds {
    Mod+X { lua-action "my_custom_action"; }
    Mod+Shift+L { lua-action "toggle_my_layout"; }
}
```

#### 3. Security Model

- Only **named, pre-registered** functions can be called (no arbitrary code execution from config)
- Functions are registered at config load time, not dynamically
- Timeout protection applies (same as event handlers)
- No access to raw key/modifier state beyond what triggered the binding

### Implementation Notes

1. Add `LuaAction(String)` variant to `niri_ipc::Action` enum
2. Add `lua-action` parsing in `niri-config/src/binds.rs`
3. Add `niri.actions:register(name, fn)` API in niri-lua
4. In action handler, look up registered function by name and execute

### Alternatives Considered

- **Expose key events**: Rejected for security (keylogging risk)
- **Inline Lua in KDL**: Rejected for complexity and security (arbitrary code in config)
- **Signal-based**: Could emit a named signal, but direct function call is simpler

---

## Review and Testing

> **For LLM Agents**: This section documents the current review and testing state of niri-lua, plus guidelines for contributing reviews and tests.

### Current Implementation State

The niri-lua crate has been largely implemented but **lacks comprehensive code review and testing**. This section provides guidelines for systematic review and test coverage expansion.

### Test Commands

```bash
# Run all niri-lua tests
cargo test --package niri-lua

# Run specific test module
cargo test --package niri-lua config_wrapper

# Run with test output visible
cargo test --package niri-lua -- --nocapture

# Run snapshot tests (uses insta)
cargo test --package niri-lua
cargo insta review  # Review snapshot changes

# Check for unused code
cargo clippy --package niri-lua --all-targets

# Verify no warnings
cargo build --package niri-lua 2>&1 | grep warning
```

### Testing Patterns

#### Unit Test Pattern

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use mlua::Lua;

    fn setup_lua() -> Lua {
        let lua = Lua::new();
        // Initialize niri global
        // ...
        lua
    }

    #[test]
    fn test_config_field_assignment() {
        let lua = setup_lua();
        lua.load(r#"
            niri.config.layout.gaps = 24
            assert(niri.config.layout.gaps == 24)
        "#).exec().unwrap();
    }

    #[test]
    fn test_collection_add_remove() {
        let lua = setup_lua();
        lua.load(r#"
            niri.config.binds:add("Mod+Q", niri.action.close_window())
            local binds = niri.config.binds:list()
            assert(#binds == 1)
            niri.config.binds:remove("Mod+Q")
            binds = niri.config.binds:list()
            assert(#binds == 0)
        "#).exec().unwrap();
    }
}
```

#### Snapshot Test Pattern

```rust
#[test]
fn test_config_serialization() {
    let lua = setup_lua();
    lua.load(r#"
        niri.config.layout.gaps = 16
        niri.config.layout.focus_ring.width = 4
    "#).exec().unwrap();

    let config = extract_config(&lua);
    insta::assert_yaml_snapshot!(config);
}
```

#### Integration Test Pattern

```rust
// In tests/repl_integration.rs
#[test]
fn test_repl_command_execution() {
    let mut repl = ReplContext::new();
    let result = repl.execute("return 1 + 1");
    assert_eq!(result, "2");
}
```

### Module Review Checklist

Use this checklist when reviewing each module:

#### `config_wrapper.rs` (Config API)

- [ ] All KDL config sections have Lua equivalents
- [ ] Type coercion matches KDL parser behavior
- [ ] Error messages are helpful and specific
- [ ] Collection proxies handle edge cases (empty, duplicate keys)
- [ ] `apply()` correctly triggers config reload

#### `action_proxy.rs` (Action System)

- [ ] All ~90 actions are registered
- [ ] Action signatures match IPC action types
- [ ] Parameterized actions validate inputs
- [ ] Error behavior matches spec (silent failure vs throw)

#### `event_data.rs` / `lua_event_hooks.rs` (Event System)

- [ ] All 14 events are wired
- [ ] Event payloads match documented types
- [ ] Handler registration/unregistration works correctly
- [ ] Memory leaks prevented (handlers cleaned up)

#### `state_query.rs` (Runtime State)

- [ ] All state queries return current data
- [ ] Return types match documented schemas
- [ ] Read-only access enforced
- [ ] Performance acceptable for frequent queries

#### `repl.rs` (REPL)

- [ ] Commands execute correctly
- [ ] Error handling provides useful output
- [ ] History works
- [ ] Multiline input handled

### Testing Priorities

Based on risk and usage frequency:

| Priority | Module | Risk Level | Rationale |
|----------|--------|------------|-----------|
| P0 | `config_wrapper.rs` | High | User-facing, complex, many edge cases |
| P0 | `action_proxy.rs` | High | Security-sensitive, external effects |
| P1 | `event_data.rs` | Medium | Memory management, callback lifecycle |
| P1 | `state_query.rs` | Medium | Data freshness, consistency |
| P2 | `repl.rs` | Low | Developer-facing, simpler scope |
| P2 | `module_loader.rs` | Low | Not yet integrated |
| P3 | `plugin_system.rs` | Low | Future feature, not active |

### Code Review Guidelines

When reviewing niri-lua code:

1. **Security**: Check for arbitrary code execution paths, sandbox escapes
2. **Memory**: Verify Lua references are properly managed (especially in event handlers)
3. **Type Safety**: Ensure Lua↔Rust type conversions handle all cases
4. **Error Handling**: Errors should be informative, not panics
5. **Performance**: Avoid allocations in hot paths (per-frame operations)
6. **Consistency**: Match patterns used elsewhere in niri codebase

### Known Gaps Requiring Attention

| Area | Gap | Priority |
|------|-----|----------|
| Config validation | No runtime validation of Lua config values | P0 |
| Action coverage | Some actions may not be fully wired | P1 |
| Event memory | Verify handler cleanup on script reload | P1 |
| Error messages | Many errors are generic | P2 |
| Documentation | Some modules lack doc comments | P2 |
| Edge cases | Collection proxies with unusual inputs | P2 |

---

## Appendix: Implementation Status

### Complete
- [x] Luau runtime with timeout protection
- [x] Full KDL configuration parity (all sections implemented)
- [x] Runtime state queries (windows, workspaces, outputs, keyboard layouts)
- [x] Event system with 14 wired events (5 categories)
- [x] ~90 actions
- [x] Timer API (with close() method)
- [x] Scheduled callbacks
- [x] REPL
- [x] Type definitions (EmmyLua)
- [x] Plugin system architecture (module loader, `require()` support)
- [x] Collection proxies with CRUD methods (add, list, get, remove, clear, set)

### Partial
- [ ] Plugin lifecycle management (`disable()`, hot-reload)

### Not Implemented
- [ ] Utility API (`niri.utils.*`) - spec complete, implementation pending
- [ ] Custom keybinding actions (`lua-action` in KDL config)
- [ ] `niri.pack` plugin manager (Git-based, vim.pack-inspired)
- [ ] Extended window queries (geometry, state)
- [ ] Custom protocol handlers

---

## Appendix: Refactor History

This section documents the major refactoring completed in February 2025 that reduced the crate by ~3,020 LOC (~17%).

### Summary

| Phase | Description | LOC Saved |
|-------|-------------|-----------|
| Phase 1.1 | EmmyLua generation refactor | -1,235 |
| Phase 1.2 | Delete unused `validators.rs` | -868 |
| Phase 2.1 | `config_field_methods!` macro | -358 |
| Phase 2.2 | `register_actions!` macro | -387 |
| Misc | Init helpers, YAGNI cleanup, quick wins | -172 |
| **Total** | | **-3,020** |

### Key Changes

#### EmmyLua Generation Refactor (Phase 1.1)

The build.rs was rewritten to generate EmmyLua type definitions from a shared schema:

| File | Before | After | Change |
|------|--------|-------|--------|
| `build.rs` | 1,779 | 534 | -1,245 |
| `api_registry.rs` | 2,516 | 348 | -2,168 |
| `api_data.rs` | 0 | 2,181 | +2,181 |
| **Net** | | | **-1,235** |

The new architecture:
- `api_data.rs` contains shared const definitions (NIRI_LUA_API schema)
- Both `api_registry.rs` and `build.rs` use `include!()` to access the schema
- EmmyLua generation is now type-safe and maintainable

#### Dead Code Removal (Phase 1.2)

`validators.rs` (868 LOC) was deleted. It contained validation logic that was implemented but never wired into the config loading pipeline. Evidence:
- Only used by its own `#[cfg(test)]` tests
- No external imports anywhere in the codebase

#### Macro Systems (Phase 2)

Two declarative macros were added to eliminate repetitive boilerplate:

**`config_field_methods!`** in `config_wrapper.rs`:
- Generates getter/setter pairs for config fields
- 25 field definitions now use the macro
- Saved ~358 LOC

**`register_actions!`** in `action_proxy.rs`:
- Registers no-argument actions in a single macro call
- ~90 actions now use the macro (1 line each instead of 3)
- Saved ~387 LOC

### Deferred Work

The following modules are intentionally kept for future Tier 5 plugin features:

| File | LOC | Purpose |
|------|-----|---------|
| `plugin_system.rs` | 716 | Plugin discovery, lifecycle, sandboxing |
| `module_loader.rs` | 276 | Custom Lua module resolution |

These are fully implemented but not yet integrated into the compositor.

### Cancelled Phases

Several proposed refactors were cancelled after analysis:

| Phase | Reason |
|-------|--------|
| `set_table_fields!` macro | Net LOC increase (macro overhead exceeded savings) |
| `LuaExtractable` trait | Only 5 primitive extractors; complex extractors are unique |
| Color conversion dedup | Already well-factored helper functions |
| Test boilerplate consolidation | Local helpers have specific hardcoded values for assertions |

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
niri.config.binds:add("Mod+Q", niri.action.close_window())
niri.config.binds:add("Mod+Left", niri.action.focus_column_left())
niri.config.binds:add("Mod+Right", niri.action.focus_column_right())
niri.config.binds:add("Mod+Up", niri.action.focus_window_up())
niri.config.binds:add("Mod+Down", niri.action.focus_window_down())
niri.config.binds:add("Mod+Shift+Left", niri.action.move_column_left())
niri.config.binds:add("Mod+Shift+Right", niri.action.move_column_right())
niri.config.binds:add("Mod+Shift+Up", niri.action.move_window_up())
niri.config.binds:add("Mod+Shift+Down", niri.action.move_window_down())

-- Workspaces
niri.config.binds:add("Mod+Page_Up", niri.action.focus_workspace_up())
niri.config.binds:add("Mod+Page_Down", niri.action.focus_workspace_down())
niri.config.binds:add("Mod+Shift+Page_Up", niri.action.move_window_to_workspace_up())
niri.config.binds:add("Mod+Shift+Page_Down", niri.action.move_window_to_workspace_down())

-- Launchers
niri.config.binds:add("Mod+Return", niri.action.spawn({"alacritty"}))
niri.config.binds:add("Mod+D", niri.action.spawn({"fuzzel"}))

-- Layout
niri.config.binds:add("Mod+F", niri.action.fullscreen_window())
niri.config.binds:add("Mod+V", niri.action.toggle_window_floating())
niri.config.binds:add("Mod+C", niri.action.center_column())
niri.config.binds:add("Mod+W", niri.action.toggle_column_tabbed())

-- Screenshots
niri.config.binds:add("Print", niri.action.screenshot())
niri.config.binds:add("Mod+Print", niri.action.screenshot_window())

-- System
niri.config.binds:add("Mod+Shift+E", niri.action.quit())
niri.config.binds:add("Mod+Shift+Slash", niri.action.show_hotkey_overlay())
niri.config.binds:add("Mod+Tab", niri.action.toggle_overview())

-- Media keys (with allow_when_locked)
niri.config.binds:add("XF86AudioRaiseVolume", {
    action = niri.action.spawn({"wpctl", "set-volume", "@DEFAULT_AUDIO_SINK@", "5%+"}),
    allow_when_locked = true,
})
niri.config.binds:add("XF86AudioLowerVolume", {
    action = niri.action.spawn({"wpctl", "set-volume", "@DEFAULT_AUDIO_SINK@", "5%-"}),
    allow_when_locked = true,
})
niri.config.binds:add("XF86AudioMute", {
    action = niri.action.spawn({"wpctl", "set-mute", "@DEFAULT_AUDIO_SINK@", "toggle"}),
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
| `module_loader.rs` | Partial | No | No | ~30% | Not yet integrated |
| `plugin_system.rs` | Partial | No | No | ~20% | Future feature |

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

### Running Coverage

```bash
# Install cargo-llvm-cov
cargo install cargo-llvm-cov

# Generate coverage report
cargo llvm-cov --package niri-lua --html

# View report
open target/llvm-cov/html/index.html
```

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

#### Intentionally Kept (Not Dead Code)

The following modules were flagged by automated analysis but are intentionally preserved:

| File | LOC | Justification |
|------|-----|---------------|
| `plugin_system.rs` | 716 | Tier 5 plugin ecosystem foundation (see Section 5) |
| `module_loader.rs` | 276 | Required for plugin system's `require()` resolution |
| `ipc_repl.rs` | 77 | Neovim-style `:lua` command via IPC (`niri msg lua "code"`) |
| `config_dirty.rs` | 161 | Granular change tracking enables future partial config reload optimization |
| `lua_types.rs` | 395 | Type definitions required for config/runtime APIs |

#### Design Decision: Keep Granular Dirty Flags

The `config_dirty.rs` module tracks 21 individual config section flags, though currently only `.any()` is called. This was intentionally kept because:

1. **Future optimization**: Enables partial config reload (only re-apply changed sections)
2. **Clean implementation**: Well-structured code with minimal maintenance burden
3. **Debugging value**: Individual flags aid in debugging config change propagation

#### Design Decision: Keep Plugin Infrastructure

The plugin system and module loader are fully implemented but not yet integrated with the compositor. These are kept because:

1. **Tier 5 roadmap**: Explicitly documented as planned feature
2. **Avoid re-implementation**: Significant work already invested
3. **Architecture commitment**: Signals direction for future development

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
