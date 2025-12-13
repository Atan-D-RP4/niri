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

### Events (28 integrated, 4 excluded for security)

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
- 28 events integrated (4 intentionally excluded for security: idle:*, key:*)

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
5. **Simple plugin model**: Neovim-style `require()` from extended `package.path`

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
| 1 | Module system, event emitter | Complete |
| 2 | Configuration API with full KDL parity | Complete |
| 3 | Runtime state queries (windows, workspaces, outputs) | Complete |
| 4 | Event system with compositor integration | Complete |
| 5 | Module loader (extends package.path for niri directories) | Complete |
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
├── module_loader.rs    # Extends package.path for niri lua directories
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
| `module_loader.rs` | 120 | Extends package.path for niri lua directories |

The module loader is integrated and extends Lua's standard `require()` mechanism.

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
| `module_loader.rs` | Yes | No | No | ~80% | Simple package.path extension |

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

#### Simplified (December 2025)

The plugin/module system was simplified to follow the Neovim model more closely:

| Change | Before | After |
|--------|--------|-------|
| `plugin_system.rs` | 716 LOC with PluginManager, metadata, lifecycle | Deleted - over-engineered |
| `module_loader.rs` | 277 LOC with custom require override | 120 LOC extending package.path |

The new approach simply extends Lua's `package.path` with niri directories, allowing standard `require()` to find modules. No custom require override, no plugin metadata, no lifecycle management.

#### Intentionally Kept (Not Dead Code)

| File | LOC | Justification |
|------|-----|---------------|
| `module_loader.rs` | 120 | Extends package.path for niri lua directories |
| `ipc_repl.rs` | 77 | Neovim-style `:lua` command via IPC (`niri msg lua "code"`) |
| `config_dirty.rs` | 161 | Granular change tracking enables future partial config reload optimization |
| `lua_types.rs` | 395 | Type definitions required for config/runtime APIs |

#### Design Decision: Keep Granular Dirty Flags

The `config_dirty.rs` module tracks 21 individual config section flags, though currently only `.any()` is called. This was intentionally kept because:

1. **Future optimization**: Enables partial config reload (only re-apply changed sections)
2. **Clean implementation**: Well-structured code with minimal maintenance burden
3. **Debugging value**: Individual flags aid in debugging config change propagation

#### Design Decision: Simple Module Loading

The module loader follows the Neovim model:

1. **Extend, don't replace**: Adds paths to `package.path`, doesn't override `require`
2. **Standard Lua semantics**: `require("foo.bar")` works exactly as expected
3. **No plugin manager**: Users load plugins explicitly via `require()` in their config
4. **Simple paths**: `~/.config/niri/lua/`, `~/.local/share/niri/lua/`, `/usr/share/niri/lua/`

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
