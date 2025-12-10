# Niri Lua Specification

Comprehensive specification for niri's Lua scripting system. This document covers the complete API surface, architecture, and implementation details.

## Table of Contents

1. [Overview](#overview)
2. [Vision](#vision)
3. [Design Decisions](#design-decisions)
4. [Architecture](#architecture)
5. [Configuration API](#configuration-api)
6. [Runtime State API](#runtime-state-api)
7. [Event System](#event-system)
8. [Action System](#action-system)
9. [Timer API](#timer-api)
10. [Async and Scheduling](#async-and-scheduling)
11. [REPL](#repl)
12. [Plugin System](#plugin-system)
13. [Type Definitions](#type-definitions)
14. [Examples](#examples)

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

- ~8,500+ lines of Rust across 15+ modules
- 400+ passing tests
- 24/24 configuration fields implemented
- 90+ actions available
- 25+ events wired

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
3. **Event System**: 25+ compositor events for reactive programming
4. **Action System**: 90+ actions for controlling the compositor
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
  - Configuration API: 24/24 fields
  - Runtime API: Windows, workspaces, outputs
  - Events: 25+ compositor events (centralized emission)
  - Actions: 90+ actions
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
pub struct ExecutionLimits {
    pub timeout: Duration,              // Default: 1 second
    pub max_scheduled_per_cycle: usize, // Default: 16
    pub max_scheduled_queue_size: usize,// Default: 1000
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
├── runtime.rs          # LuaRuntime core (1389 lines)
├── config_proxy.rs     # niri.config API
├── config_converter.rs # Lua→Config conversion (4250 lines)
├── action_proxy.rs     # niri.action API (~90 actions)
├── events_proxy.rs     # niri.events API
├── runtime_api.rs      # niri.state API (910 lines)
├── loop_api.rs         # niri.loop timer API
├── repl.rs             # Interactive REPL
├── event_emitter.rs    # Core event infrastructure
├── api_registry.rs     # API namespace registration
├── collections.rs      # Lua collection utilities
├── error.rs            # Error types
├── utils.rs            # Shared utilities
└── types/api.lua       # Luau type definitions

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
pub struct ExecutionLimits {
    pub timeout: Duration,              // Default: 1 second
    pub max_scheduled_per_cycle: usize, // Default: 16
    pub max_scheduled_queue_size: usize,// Default: 1000
}
```

---

## Configuration API

The `niri.config` namespace provides full KDL configuration parity. Configurations are specified as Lua tables and converted to niri's internal `Config` struct.

### Basic Usage

```lua
local niri = require("niri")

niri.config.input.keyboard.xkb.layout = "us"
niri.config.input.touchpad.tap = true

niri.config.layout.gaps = 16
niri.config.layout.center_focused_column = "never"

niri.config.binds["Mod+Return"] = niri.action.spawn("alacritty")
niri.config.binds["Mod+Q"] = niri.action.close_window()
```

### Configuration Fields (24/24)

| Field | Type | Description |
|-------|------|-------------|
| `input` | table | Input device configuration |
| `outputs` | table | Monitor configuration |
| `layout` | table | Tiling layout settings |
| `spawn_at_startup` | array | Commands to run at startup |
| `prefer_no_csd` | boolean | Prefer server-side decorations |
| `screenshot_path` | string | Screenshot save location |
| `hotkey_overlay` | table | Hotkey overlay settings |
| `environment` | table | Environment variables |
| `cursor` | table | Cursor theme and size |
| `binds` | table | Key bindings |
| `window_rules` | array | Window matching rules |
| `layer_rules` | array | Layer surface rules |
| `debug` | table | Debug options |
| `animations` | table | Animation settings |
| `gestures` | table | Touchpad gestures |
| `overview` | table | Overview mode settings |
| `switch_events` | table | Lid/tablet switch handling |

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
    },
    mouse = {
        natural_scroll = false,
        accel_speed = 0.0,
        accel_profile = "flat",
        scroll_button = 274, -- middle button
        scroll_button_lock = false,
    },
    tablet = {
        map_to_output = "eDP-1",
    },
    touch = {
        map_to_output = "eDP-1",
    },
    power_key_handling = "suspend", -- "suspend" | "ignore"
    disable_power_key_handling = false,
    warp_mouse_to_focus = false,
    focus_follows_mouse = false,
    workspace_auto_back_and_forth = false,
}
```

### Output Configuration

```lua
niri.config.outputs["eDP-1"] = {
    mode = {
        width = 1920,
        height = 1080,
        refresh = 60.0,
    },
    scale = 1.0,
    transform = "normal", -- "normal" | "90" | "180" | "270" | "flipped" | "flipped-90" | "flipped-180" | "flipped-270"
    position = { x = 0, y = 0 },
    variable_refresh_rate = false,
    background_color = "#000000",
}
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
        width = 4,
        active_color = "#7fc8ff",
        inactive_color = "#505050",
        -- OR use gradient:
        active_gradient = {
            from = "#80c8ff",
            to = "#bbddff",
            angle = 45,
            relative_to = "workspace-view", -- "workspace-view" | "window"
        },
    },
    border = {
        enable = true,  -- replaces focus_ring when enabled
        width = 4,
        active_color = "#ffc87f",
        inactive_color = "#505050",
    },
    shadow = {
        enable = true,
        softness = 30,
        spread = 5,
        offset = { x = 0, y = 5 },
        color = "#00000070",
        inactive_color = "#00000040",
    },
    tab_indicator = {
        hide_when_single_tab = true,
        gap = 10.0,
        width = 4.0,
        length = { proportion = 0.3 },
        corner_radius = 8.0,
        active_color = "#ffc87f",
        inactive_color = "#505050",
    },
    insert_hint = {
        enable = true,
        color = "#ffc87f80",
    },
}
```

### Key Bindings

```lua
-- Simple bindings
niri.config.binds["Mod+Return"] = niri.action.spawn("alacritty")
niri.config.binds["Mod+Q"] = niri.action.close_window()

-- Bindings with modifiers
niri.config.binds["Mod+Shift+E"] = niri.action.quit()
niri.config.binds["Mod+Ctrl+L"] = niri.action.spawn("swaylock")

-- Allow when locked
niri.config.binds["XF86AudioRaiseVolume"] = {
    action = niri.action.spawn("wpctl", "set-volume", "@DEFAULT_AUDIO_SINK@", "5%+"),
    allow_when_locked = true,
}

-- Cooldown (prevent rapid re-triggering)
niri.config.binds["Mod+Tab"] = {
    action = niri.action.focus_window_down_or_column_right(),
    cooldown_ms = 150,
}
```

### Window Rules

```lua
niri.config.window_rules = {
    {
        matches = {
            { app_id = "firefox" },
            { app_id = "chromium" },
        },
        default_column_width = { proportion = 0.7 },
    },
    {
        matches = {
            { app_id = ".*dialog.*", is_regex = true },
        },
        open_floating = true,
    },
    {
        matches = {
            { title = "Picture-in-Picture" },
        },
        open_floating = true,
        default_floating_position = { x = 20, y = 20, relative_to = "top-right" },
    },
}
```

### Animations

```lua
niri.config.animations = {
    slowdown = 1.0,  -- Global slowdown factor
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
    },
    config_notification_open_close = {
        spring = {
            damping_ratio = 0.6,
            stiffness = 1000,
            epsilon = 0.001,
        },
    },
}
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

### API Methods

```lua
-- Subscribe to event
local id = niri.events:on("window:open", function(event)
    print("Window opened:", event.app_id, event.title)
end)

-- Subscribe once (auto-unsubscribes after first call)
niri.events:once("window:focus", function(event)
    print("First focus:", event.app_id)
end)

-- Unsubscribe
niri.events:off("window:open", id)

-- Emit custom event (for testing/plugins)
niri.events:emit("custom_event", { data = "value" })

-- List registered events
local events = niri.events:list()

-- Clear all handlers for an event
niri.events:clear("window:open")
```

### Available Events

#### Window Events
| Event | Payload | Description |
|-------|---------|-------------|
| `window:open` | `{id, app_id, title, workspace_id}` | Window created |
| `window:close` | `{id, app_id, title}` | Window destroyed |
| `window:focus` | `{id, app_id, title}` | Window gained focus |
| `window:blur` | `{id, title}` | Window lost focus |
| `window:title_changed` | `{id, title}` | Window title updated |
| `window:app_id_changed` | `{id, app_id}` | Window app_id updated |
| `window:fullscreen` | `{id, is_fullscreen}` | Fullscreen state changed |
| `window:maximize` | `{id, is_maximized}` | Maximize state changed |
| `window:move` | `{id, workspace_id}` | Window moved to workspace |
| `window:resize` | `{id, width, height}` | Window resized |

#### Workspace Events
| Event | Payload | Description |
|-------|---------|-------------|
| `workspace:create` | `{id, idx, name, output}` | Workspace created |
| `workspace:destroy` | `{id, name, output}` | Workspace destroyed |
| `workspace:activate` | `{id, idx, name, output}` | Workspace became active |
| `workspace:deactivate` | `{id, idx, name, output}` | Workspace became inactive |
| `workspace:rename` | `{id, name, old_name}` | Workspace renamed |

#### Layout Events
| Event | Payload | Description |
|-------|---------|-------------|
| `layout:window_added` | `{window_id, workspace_id}` | Window added to layout |
| `layout:window_removed` | `{window_id, workspace_id}` | Window removed from layout |
| `layout:mode_changed` | `{mode}` | Layout mode changed |

#### Output Events
| Event | Payload | Description |
|-------|---------|-------------|
| `output:mode_change` | `{name, width, height, refresh}` | Output mode changed |

#### Monitor Events
| Event | Payload | Description |
|-------|---------|-------------|
| `monitor:connect` | `{name, make, model}` | Monitor connected |
| `monitor:disconnect` | `{name}` | Monitor disconnected |

#### Config Events
| Event | Payload | Description |
|-------|---------|-------------|
| `config:reload` | `{}` | Configuration reloaded |

#### Overview Events
| Event | Payload | Description |
|-------|---------|-------------|
| `overview:open` | `{}` | Overview mode opened |
| `overview:close` | `{}` | Overview mode closed |

#### Lock Events
| Event | Payload | Description |
|-------|---------|-------------|
| `lock:activate` | `{}` | Session locked |
| `lock:deactivate` | `{}` | Session unlocked |

#### Lifecycle Events
| Event | Payload | Description |
|-------|---------|-------------|
| `startup` | `{}` | Compositor started |
| `shutdown` | `{}` | Compositor shutting down |

### Intentionally Unimplemented Events

The following events are defined in the API but **intentionally not wired** for security reasons:

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

### Basic Usage

```lua
-- Execute immediately
niri.action.close_window()
niri.action.spawn("alacritty")

-- Store for later
local my_action = niri.action.focus_workspace_down()

-- Use in bindings
niri.config.binds["Mod+Q"] = niri.action.close_window()
```

### Available Actions (90+)

#### Application Actions
```lua
niri.action.quit()
niri.action.quit_skip_confirmation()
niri.action.spawn("command", "arg1", "arg2", ...)
niri.action.spawn_at_startup("command", "arg1", ...)  -- Runs at niri start
```

#### Window Actions
```lua
niri.action.close_window()
niri.action.focus_window_up()
niri.action.focus_window_down()
niri.action.focus_window_left()
niri.action.focus_window_right()
niri.action.focus_window_up_or_column_left()
niri.action.focus_window_down_or_column_right()
niri.action.move_window_up()
niri.action.move_window_down()
niri.action.move_window_up_or_to_workspace_up()
niri.action.move_window_down_or_to_workspace_down()
niri.action.consume_or_expel_window_left()
niri.action.consume_or_expel_window_right()
niri.action.consume_window_into_column()
niri.action.expel_window_from_column()
```

#### Column Actions
```lua
niri.action.focus_column_left()
niri.action.focus_column_right()
niri.action.focus_column_first()
niri.action.focus_column_last()
niri.action.focus_column_right_or_first()
niri.action.focus_column_left_or_last()
niri.action.move_column_left()
niri.action.move_column_right()
niri.action.move_column_left_or_to_monitor_left()
niri.action.move_column_right_or_to_monitor_right()
niri.action.move_column_to_first()
niri.action.move_column_to_last()
niri.action.center_column()
```

#### Workspace Actions
```lua
niri.action.focus_workspace_up()
niri.action.focus_workspace_down()
niri.action.focus_workspace(idx)          -- 1-based index
niri.action.focus_workspace("name")       -- By name
niri.action.focus_workspace_previous()
niri.action.move_window_to_workspace_up()
niri.action.move_window_to_workspace_down()
niri.action.move_window_to_workspace(idx)
niri.action.move_window_to_workspace("name")
niri.action.move_column_to_workspace_up()
niri.action.move_column_to_workspace_down()
niri.action.move_column_to_workspace(idx)
niri.action.move_workspace_up()
niri.action.move_workspace_down()
niri.action.move_workspace_to_index(idx)
```

#### Monitor Actions
```lua
niri.action.focus_monitor_left()
niri.action.focus_monitor_right()
niri.action.focus_monitor_up()
niri.action.focus_monitor_down()
niri.action.focus_monitor("name")         -- By output name
niri.action.focus_monitor_next()
niri.action.focus_monitor_previous()
niri.action.move_window_to_monitor_left()
niri.action.move_window_to_monitor_right()
niri.action.move_window_to_monitor("name")
niri.action.move_column_to_monitor_left()
niri.action.move_column_to_monitor_right()
niri.action.move_column_to_monitor("name")
niri.action.move_workspace_to_monitor_left()
niri.action.move_workspace_to_monitor_right()
niri.action.move_workspace_to_monitor("name")
```

#### Size Actions
```lua
-- Proportional
niri.action.set_column_width({ proportion = 0.5 })
niri.action.set_window_height({ proportion = 0.75 })

-- Fixed pixels
niri.action.set_column_width({ fixed = 800 })
niri.action.set_window_height({ fixed = 600 })

-- Relative changes
niri.action.set_column_width("+10%")      -- Increase by 10%
niri.action.set_column_width("-100")      -- Decrease by 100px
niri.action.set_window_height("+50")

-- Switching presets
niri.action.switch_preset_column_width()
niri.action.switch_preset_window_height()
niri.action.reset_window_height()
niri.action.maximize_column()
niri.action.fullscreen_window()
```

#### Layout Actions
```lua
niri.action.toggle_column_tabbed()
niri.action.switch_layout("next")         -- "next" | "prev"
niri.action.switch_focus_between_floating_and_tiling()
niri.action.toggle_window_floating()
```

#### Screenshot Actions
```lua
niri.action.screenshot()                  -- Full screen
niri.action.screenshot_screen()           -- Current screen
niri.action.screenshot_window()           -- Focused window
```

#### Output Actions
```lua
niri.action.power_off_monitors()
niri.action.power_on_monitors()
niri.action.toggle_debug_tint()
```

#### Misc Actions
```lua
niri.action.toggle_overview()
niri.action.do_screen_transition()
niri.action.show_hotkey_overlay()
niri.action.confirm_screenshot()           -- Used with interactive screenshots
```

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

-- Stop timer
timer:stop()

-- Restart with same settings
timer:again()

-- Check if timer is active
local active = timer:is_active()
```

### Get Current Time

```lua
-- Get current time in milliseconds (since compositor start)
local now = niri.loop.now()
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

niri.events:on("window_opened", function(event)
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

**Limits:**
- Maximum 16 callbacks processed per cycle
- Maximum 1000 callbacks in queue
- Exceeding limits drops oldest callbacks

### Timeout Protection

All Lua execution is protected by configurable timeouts:

```lua
-- Default: 1 second timeout
-- Long-running code will be interrupted:

niri.events:on("window_opened", function(event)
    -- This will timeout and be interrupted:
    for i = 1, 1e12 do end
end)
```

The Luau dialect's `set_interrupt` mechanism enables reliable interruption without undefined behavior.

### Processing Cycle

Each frame, `process_async()` handles:
1. Scheduled callbacks (up to `max_scheduled_per_cycle`)
2. Timer callbacks (expired timers)

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
niri> niri.action.spawn("wofi", "--show", "drun")
niri> niri.events:list()
{"window_opened", "window_closed", "window_focused", ...}
```

### REPL Commands

```
niri> :help      -- Show help
niri> :quit      -- Exit REPL
niri> :clear     -- Clear screen
```

---

## Plugin System

### Plugin Discovery

Plugins are discovered from standard XDG directories:

```
~/.config/niri/plugins/*.lua
/etc/xdg/niri/plugins/*.lua
```

### Plugin Structure

```lua
-- ~/.config/niri/plugins/my-plugin.lua

local M = {}

function M.setup(opts)
    opts = opts or {}
    
    -- Register event handlers
    niri.events:on("window:open", function(event)
        -- Plugin logic
    end)
    
    -- Return public API
    return M
end

return M
```

### Loading Plugins

```lua
-- In niri.lua config
local my_plugin = require("my-plugin")
my_plugin.setup({ option = "value" })
```

### Plugin Best Practices

1. **Keep handlers fast**: Respect timeout limits
2. **Clean up on unload**: Unsubscribe from events
3. **Namespace globals**: Use local modules
4. **Document configuration**: Provide clear options

### Sandboxing (Planned)

Future versions will include:
- Resource limits per plugin
- Capability-based permissions
- Isolated Lua states

---

## Type Definitions

Luau type definitions are provided in `niri-lua/types/api.lua` for IDE support.

### Usage with Luau LSP

```lua
--!strict

-- Type annotations are inferred from the definitions
local windows = niri.state.windows()  -- Inferred as {Window}
```

### Key Types

```lua
export type Window = {
    id: number,
    app_id: string,
    title: string,
    workspace_id: number,
    is_focused: boolean,
    is_floating: boolean,
}

export type Workspace = {
    id: number,
    idx: number,
    name: string?,
    output: string,
    is_active: boolean,
    is_focused: boolean,
    active_window_id: number?,
}

export type Output = {
    name: string,
    make: string,
    model: string,
    serial: string,
    width: number,
    height: number,
    refresh: number,
    scale: number,
    transform: string,
    is_enabled: boolean,
}

export type SizeChange = 
    | { proportion: number }
    | { fixed: number }
    | string  -- "+10%", "-100", etc.

export type Timer = {
    start: (self: Timer, delay_ms: number, repeat_ms: number, callback: () -> ()) -> (),
    stop: (self: Timer) -> (),
    again: (self: Timer) -> (),
    is_active: (self: Timer) -> boolean,
}
```

---

## Examples

### Auto-move Windows by App

```lua
-- Move specific apps to designated workspaces
niri.events:on("window_opened", function(event)
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
niri.events:on("workspace_switched", function(event)
    local name = event.name or ("Workspace " .. event.idx)
    print("Switched to: " .. name)
end)
```

### Focus Follows Mouse (Custom)

```lua
-- Custom focus-follows-mouse with delay
local focus_timer = niri.loop.new_timer()
local pending_window = nil

niri.events:on("pointer_entered_window", function(event)
    pending_window = event.window_id
    focus_timer:stop()
    focus_timer:start(150, 0, function()
        if pending_window then
            niri.action.focus_window_by_id(pending_window)
            pending_window = nil
        end
    end)
end)
```

### Window Counter

```lua
-- Track window count
local window_count = 0

niri.events:on("window_opened", function()
    window_count = window_count + 1
    print("Windows: " .. window_count)
end)

niri.events:on("window_closed", function()
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

niri.events:on("window_opened", update_gaps)
niri.events:on("window_closed", update_gaps)
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
    niri.action.spawn("alacritty", "--class", "scratchpad")
end

niri.events:on("window_opened", function(event)
    if event.app_id == "scratchpad" then
        scratchpad_id = event.id
        niri.action.toggle_window_floating()
    end
end)

niri.config.binds["Mod+grave"] = toggle_scratchpad
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
        niri.action:move_window_to_workspace(2)
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

## Appendix: Implementation Status

### Complete
- [x] Luau runtime with timeout protection
- [x] Full KDL configuration parity (24/24 fields)
- [x] Runtime state queries
- [x] Event system with 25+ events
- [x] 90+ actions
- [x] Timer API
- [x] Scheduled callbacks
- [x] REPL
- [x] Type definitions

### Partial
- [ ] Plugin sandboxing
- [ ] Plugin lifecycle management
- [ ] Resource limits per plugin

### Planned
- [ ] Custom keybinding actions (`lua-action` in KDL config)
- [ ] Hot-reload plugins
- [ ] Plugin marketplace/registry
- [ ] Custom protocol handlers
- [ ] Extended window queries (geometry, state)
