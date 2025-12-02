# Session Document: Lua API Refactor Implementation

## Quick Start for New Agents

This document provides all context needed to continue implementing the Niri Lua API Refactor. Read this document first, then use the referenced files to build deeper context as needed.

## Project Overview

**Niri** is a scrollable-tiling Wayland compositor written in Rust. It has a Lua scripting system (`niri-lua` crate) that allows users to configure and extend the compositor.

**Goal**: Implement a reactive configuration system where Lua scripts can modify compositor settings at runtime via `niri.config.*` with explicit apply semantics.

---

## Current Status

| Phase | Status | Description |
|-------|--------|-------------|
| R1: Core Infrastructure | ✅ COMPLETE | Proxy tables, PendingConfigChanges, collection CRUD |
| R2: IPC Integration | ✅ COMPLETE | Apply pending changes to live compositor |
| R3: Namespace Migration | ✅ COMPLETE | Reorganize `niri.state`, `niri.utils`, etc. |
| R4: Event System Refactor | ✅ COMPLETE | New `niri.events:on()` API |
| R5: Action System | ✅ COMPLETE | `niri.action.*` (~90 actions) |
| R6: New Events | ✅ COMPLETE | High-priority events added |
| R7: Cleanup | ✅ COMPLETE | Removed old APIs, updated docs |
| R8: Config Loader Enhancement | ✅ COMPLETE | Make `-c file.lua` recognize Lua files |
| R9: Reactive Config Read | ✅ COMPLETE | Populate proxies with actual config values |
| R10: Reactive Config Write | ✅ COMPLETE | Make `apply()` update compositor state |
| R11: Collection CRUD | ✅ COMPLETE | Full binds/window_rules CRUD operations |
| R12: Legacy Code Cleanup | ✅ COMPLETE | Remove obsolete old global-variable API |
| R13: Runtime Config Application | ✅ COMPLETE | Complete runtime config change pipeline for all sections |

**Test Status**: All tests passing
- `cargo test -p niri-lua --lib` → 440 unit tests
- `cargo test -p niri-lua --test repl_integration` → 104 integration tests

---

## Phase R6 Implementation Summary (COMPLETE)

### New Events Added

The following events are now implemented and wired up in the compositor:

1. **Window events**:
   - `window:title_changed` - Emitted when a window's title changes
     - Data: `{ id = number, title = string }`
   - `window:app_id_changed` - Emitted when a window's app_id changes
     - Data: `{ id = number, app_id = string }`
   - `window:fullscreen` - Emitted when a window enters/exits fullscreen
     - Data: `{ id = number, title = string, is_fullscreen = boolean }`

2. **System events**:
   - `config:reload` - Emitted when configuration is reloaded
     - Data: `{ success = boolean }`
   - `overview:open` - Emitted when overview mode is opened
     - Data: `{ is_open = true }`
   - `overview:close` - Emitted when overview mode is closed
     - Data: `{ is_open = false }`

### Implementation Files Modified

| File | Changes |
|------|---------|
| `src/lua_event_hooks.rs` | Added 6 new emit functions + helper functions |
| `src/handlers/xdg_shell.rs` | Added event emissions in `title_changed`, `app_id_changed`, `fullscreen_request`, `unfullscreen_request` |
| `src/niri.rs` | Added `config:reload` event emission in `reload_config` |
| `src/input/mod.rs` | Added `overview:open/close` events in action handlers |

### Deferred Events (Future Work)

The following events require deeper architectural changes and are deferred:

- `window:move` - Requires tracking window movement through layout operations
- `workspace:create` / `workspace:destroy` - Layout module lacks access to State
- `workspace:rename` - Same architectural constraint

These would require either:
1. Callback-based architecture in the layout module, or
2. Event queue that collects events from layout and emits them when State is available

---

## Phase R7 Implementation Summary (COMPLETE)

### Changes Made

1. **Removed backward-compatible aliases** from `niri-lua/src/niri_api.rs`:
   - Removed top-level `niri.log`, `niri.debug`, `niri.warn`, `niri.error`, `niri.spawn`
   - Users should now use `niri.utils.*` namespace

2. **Removed legacy event API** from `niri-lua/src/runtime.rs`:
   - Removed `niri.on()`, `niri.once()`, `niri.off()` registration
   - Users should now use `niri.events:on/once/off()`

3. **Removed `register_event_api_to_lua()` function** from `niri-lua/src/event_system.rs`:
   - This function was only used for the legacy API

4. **Updated exports** in `niri-lua/src/lib.rs`:
   - Removed `register_event_api_to_lua` from public exports

5. **Updated tests**:
   - Fixed 3 unit tests in `niri_api.rs` and `config.rs` to use new API
   - Removed `test_events_proxy_backward_compat_with_legacy_api` integration test

6. **Updated documentation**:
   - `docs/LUA_GUIDE.md` - All examples now use `niri.utils.*` namespace
   - `docs/LUA_EVENT_HOOKS.md` - All examples now use `niri.events:*` methods

### Final API Surface

```lua
-- Utilities (was niri.log, etc.)
niri.utils.log("message")
niri.utils.debug("message")
niri.utils.warn("message")
niri.utils.error("message")
niri.utils.spawn("command")
niri.utils.spawn_blocking("command")

-- Events (was niri.on, etc.)
local id = niri.events:on("event:name", callback)
niri.events:once("event:name", callback)
niri.events:off("event:name", id)
niri.events:emit("event:name", data)
niri.events:list()
niri.events:clear("event:name")

-- Actions
niri.action:quit()
niri.action:spawn("command")
-- ... ~90 actions

-- State queries
niri.state.windows()
niri.state.workspaces()
niri.state.focused_window()
-- etc.

-- Configuration
niri.config.set_animations({...})
niri.config.get_animations()
-- etc.
```

---

---

## Phase R8: Config Loader Enhancement (COMPLETE)

**Goal**: Make the config loader recognize `.lua` files passed via `-c` flag.

**Implementation**: Modified `niri-config/src/lib.rs` to detect `.lua` file extension during config loading. When detected, a flag is set that signals to skip KDL parsing and instead route to Lua runtime execution.

**Key Changes**:
- `niri-config/src/lib.rs` - Added Lua file extension detection
- Lua config files now load via `LuaRuntime` instead of KDL parser
- `niri -c examples/niriv2.lua` now works correctly

---

## Phase R9-R11: Reactive Config Implementation (COMPLETE)

**Goal**: Implement full reactive configuration system where Lua scripts can modify compositor settings at runtime via `niri.config.*` with explicit apply semantics.

**Implementation Summary**:

Created `apply_pending_lua_config()` function in `niri-lua/src/config_converter.rs` (~4000 lines) that applies `PendingConfigChanges` from the reactive API to actual Niri config.

### Scalar Sections Supported

| Section | Fields |
|---------|--------|
| `layout` | gaps, struts, focus_ring, border, shadow, colors, presets (center_focused_column, always_center_focused_column, default_column_width, preset_column_widths, preset_window_heights) |
| `input.keyboard` | xkb.layout, xkb.model, xkb.rules, xkb.variant, xkb.options, repeat_delay, repeat_rate, track_layout |
| `input.touchpad` | natural_scroll, accel_speed, accel_profile, scroll_method, tap, tap_button_map, tap_drag, tap_drag_lock, dwt, dwtp, left_handed, click_method, middle_emulation, disabled_on_external_mouse, off |
| `input.mouse` | natural_scroll, accel_speed, accel_profile, left_handed, scroll_method, middle_emulation, off |
| `input.trackpoint` | natural_scroll, accel_speed, accel_profile, left_handed, scroll_method, middle_emulation, off |
| `input.touch` | natural_scroll, off, map_to_output |
| `cursor` | size, theme, hide_when_typing, hide_after_inactive_ms |
| `animations` | Global settings and per-animation overrides |
| `gestures` | workspace_swipe (enabled, fingers, threshold, natural_scroll, etc.) |
| `overview` | backdrop_color, zoom |
| `recent_windows` | backdrop_color |
| `clipboard` | history_size, disable_autosave |
| `hotkey_overlay` | skip_at_startup |
| `config_notification` | disabled |
| `debug` | Various debug flags |
| `xwayland_satellite` | binary, env, lib_gl, socket_dir |
| Global | prefer_no_csd, screenshot_path |

### Collection Operations Supported

| Collection | Operations |
|------------|------------|
| `binds` | Add keybindings with key, action, modifiers, allow_when_locked, cooldown |
| `window_rules` | Add rules with singular `match` format (app_id, title, is_floating, etc.) |
| `outputs` | Add output configurations (mode, scale, position, transform, etc.) |
| `spawn_at_startup` | Add startup commands captured from `niri.action:spawn()` during config load |
| `environment` | Add environment variable definitions |
| `workspaces` | Add named workspace definitions |

### Helper Functions

- `json_to_color()` - Parse hex colors (#RRGGBB or #RRGGBBAA)
- `json_to_preset_size()` - Parse proportion/fixed sizes
- `json_to_output()` - Parse full output configuration
- `json_to_spawn()` - Parse spawn command with environment
- `json_to_environment()` - Parse environment variable
- `json_to_workspace()` - Parse workspace definition
- `json_to_window_rule()` - Parse window rule with match criteria
- `json_to_bind()` - Parse keybinding specification

### Startup Command Capture

Modified `niri-lua/src/runtime.rs` to capture `niri.action:spawn()` calls during config loading:
- `init_empty_config_proxy()` now intercepts spawn actions
- Spawn commands added to `PendingConfigChanges` as `spawn_at_startup` entries

### Test Results

```
Applying pending Lua config: 47 scalar changes, 117 collection additions
Applied 150 pending Lua config changes, now 102 binds total
```
- 111 keybindings, 3 spawn_at_startup, 3 outputs all working
- XKB options applied, event handlers work

**Key Files**:
- `niri-lua/src/config_converter.rs` - Main converter with `apply_pending_lua_config()`
- `niri-lua/src/runtime.rs` - Lua runtime with spawn capture
- `niri-lua/src/config_proxy.rs` - `PendingConfigChanges` struct
- `examples/niriv2.lua` - Test config using reactive API

---

## Phase R12: Legacy Code Cleanup (COMPLETE)

**Goal**: Remove obsolete old global-variable-based API code now that the reactive API is complete.

The codebase now has two parallel paths - the reactive API is complete, so the old global-variable-based API can be removed.

### Files to Modify

#### 1. `niri-lua/src/config_converter.rs` (~2500 lines to remove)

**Remove**:
- `apply_lua_config()` function (lines ~513-2535) - reads global variables from Lua state
- Old helper functions (lines ~41-511):
  - `lua_keybinding_to_bind()`
  - `parse_hex_color()` 
  - `parse_preset_size()`
  - `parse_border_config()`
  - `parse_shadow_config()`
  - `parse_gradient_extra_stop()`
  - `parse_gradient_config()`
  - `parse_focus_ring_config()`
  - And other legacy parsing functions

**Keep**:
- `apply_pending_lua_config()` and its helper functions (`json_to_*`)

#### 2. `niri-lua/src/runtime.rs` - Global variable methods to remove

**Remove**:
- `has_global()`
- `get_global_string_opt()`
- `get_global_bool_opt()`
- `get_global_int_opt()`
- `get_global_table_opt()`
- `get_binds()` - old method reading `BINDS` global
- `get_startup_commands()` - old method reading `SPAWN_AT_STARTUP` global

**Keep**:
- `get_pending_config_changes()` - for reactive API
- All other runtime methods

#### 3. `niri-lua/src/lib.rs` - Update exports

**Remove**:
- `apply_lua_config` from public exports

**Keep**:
- `apply_pending_lua_config` export

#### 4. `src/main.rs` - Simplify config loading

**Remove**:
- Call to `apply_lua_config()` 

**Keep**:
- Only `apply_pending_lua_config()` call

### Migration Notes

- No backwards compatibility needed - the old API was internal
- All tests should be updated to use reactive API patterns
- `examples/niriv2.lua` demonstrates the new API

### Validation Commands

```bash
# After cleanup, verify all tests pass
cargo test -p niri-lua --lib
cargo test -p niri-lua --test repl_integration
cargo build

# Test Lua config loading
niri -c examples/niriv2.lua
```

---

## Phase R13: Runtime Config Application (COMPLETE)

**Goal**: Complete the runtime config change pipeline so `niri msg lua "niri.config.X = Y"` applies changes to the live compositor.

**Problem**: Currently, `State::apply_pending_lua_config()` in `src/niri.rs` only handles `layout.*` and `animations.*` sections. Other scalar config changes (like `prefer_no_csd`, `input.*`, `cursor.*`, etc.) fall through to the debug log and are not applied.

### Analysis: Remove Unused apply() Machinery

The `niri.config:apply()` and `niri.config:auto_apply()` methods in `config_proxy.rs` are **unused stubs**:

1. **IPC server already auto-applies** - After executing Lua code via IPC, `server.rs:468` automatically calls `state.apply_pending_lua_config()`
2. **`apply()` just logs** - The method at `config_proxy.rs:502-513` only logs debug messages, doesn't actually apply
3. **`auto_apply` field unused** - The `PendingConfigChanges.auto_apply` field is set but never checked
4. **No Lua code uses them** - No `.apply()` calls found in any Lua files

**Decision**: Remove the unused `apply()` and `auto_apply()` machinery. The auto-apply after IPC execution is the correct pattern.

### Implementation Approach

Instead of path-by-path handling for each config section, use the **full reload approach**:

1. **Merge pending changes into Config** - Apply all `scalar_changes` to the `Config` struct
2. **Leverage existing `reload_config()` machinery** - Reuse the proven logic for XKB, libinput, cursor, outputs, etc.
3. **Less code, complete coverage** - New config sections automatically work

### Tasks

#### Part A: Remove Unused Machinery

**File: `niri-lua/src/config_proxy.rs`**

Remove:
- `apply()` method (lines ~502-513)
- `auto_apply()` method (lines ~296-329)
- `auto_apply` field from `PendingConfigChanges` struct
- Related tests

**File: `niri-lua/src/lib.rs`**

Remove any exports related to `apply()` if present.

#### Part B: Complete apply_pending_lua_config()

**File: `src/niri.rs` in `State::apply_pending_lua_config()`**

Modify to:
1. Iterate through all `pending.scalar_changes`
2. For each change, update the corresponding field in `config`
3. After all changes applied, call relevant parts of `reload_config()`:
   - `update_xkb_config()` if keyboard settings changed
   - Libinput device reconfiguration if input settings changed
   - `backend.set_cursor_*()` if cursor settings changed
   - `layout.update_config()` (already done)
   - `queue_redraw_all()` if visual settings changed

### Config Sections to Support

| Section | Fields | Reload Action |
|---------|--------|---------------|
| `prefer_no_csd` | boolean | Queue redraw |
| `screenshot_path` | string/null | None needed |
| `input.keyboard.*` | xkb.*, repeat_* | `update_xkb_config()` |
| `input.touchpad.*` | natural_scroll, accel_*, etc. | Libinput reconfigure |
| `input.mouse.*` | natural_scroll, accel_*, etc. | Libinput reconfigure |
| `input.trackpoint.*` | natural_scroll, accel_*, etc. | Libinput reconfigure |
| `input.touch.*` | map_to_output, off | Libinput reconfigure |
| `cursor.*` | theme, size, hide_* | `backend.set_cursor_*()` |
| `gestures.*` | workspace_swipe.* | None needed |
| `overview.*` | backdrop_color, zoom | Queue redraw |
| `recent_windows.*` | backdrop_color | Queue redraw |
| `clipboard.*` | history_size, disable_autosave | None needed |
| `hotkey_overlay.*` | skip_at_startup | None needed |
| `config_notification.*` | disabled | None needed |
| `debug.*` | various flags | Queue redraw |
| `xwayland_satellite.*` | binary, env, etc. | None needed (next launch) |

### Validation

```bash
# Test runtime config changes (no explicit apply() needed!)
niri msg lua "niri.config.prefer_no_csd = false"
niri msg lua "niri.config.layout.gaps = 20"
niri msg lua "niri.config.cursor.size = 32"

# Batch multiple changes
niri msg lua '
  niri.config.prefer_no_csd = false
  niri.config.cursor.size = 32
  niri.config.layout.gaps = 20
'
# All applied automatically after IPC execution
```

---

## Future Event Candidates (Deferred)

These events require deeper architectural changes and are deferred to future work:

1. **Window events**:
   - `window:move` - When window moves to different workspace/monitor

2. **Workspace events**:
   - `workspace:create` - When a new workspace is created
   - `workspace:destroy` - When a workspace is removed
   - `workspace:rename` - When workspace name changes

These would require either callback-based architecture in the layout module, or an event queue pattern.

### Implementation Pattern

To add a new event, follow these steps:

1. **Define event data** in `niri-lua/src/event_data.rs`:
```rust
pub fn window_title_changed_data(lua: &Lua, window_id: u64, old_title: &str, new_title: &str) -> LuaResult<LuaTable> {
    let table = lua.create_table()?;
    table.set("window_id", window_id)?;
    table.set("old_title", old_title)?;
    table.set("new_title", new_title)?;
    Ok(table)
}
```

2. **Add emission helper** in `src/lua_event_hooks.rs`:
```rust
pub fn emit_window_title_changed(niri: &mut Niri, window_id: u64, old_title: &str, new_title: &str) {
    if let Some(ref mut runtime) = niri.lua_runtime {
        if let Some(ref event_system) = runtime.event_system {
            let _ = event_system.emit("window:title_changed", |lua| {
                event_data::window_title_changed_data(lua, window_id, old_title, new_title)
            });
        }
    }
}
```

3. **Call the helper** from the compositor where the event occurs (e.g., in `src/window/mapped.rs` when title changes)

4. **Add tests** in `niri-lua/tests/repl_integration.rs`

### Key Files for Phase R6

| File | Purpose |
|------|---------|
| `niri-lua/src/event_data.rs` | Event data converters (~706 lines) |
| `niri-lua/src/event_handlers.rs` | Handler registry (~328 lines) |
| `niri-lua/src/event_system.rs` | Thread-safe event emission (~243 lines) |
| `src/lua_event_hooks.rs` | Compositor-side event emission helpers |
| `src/window/mapped.rs` | Window state changes happen here |
| `src/layout/workspace.rs` | Workspace changes happen here |

### Validation

Run after implementation:
```bash
cargo test -p niri-lua
cargo test -p niri --lib
```

---

## Phase R4-R5 Implementation Summary (Just Completed)

### Phase R4: Event System Refactor (COMPLETE)

**File: `niri-lua/src/events_proxy.rs`** (~200 lines)

Created `EventsProxy` userdata implementing:
- `niri.events:on(event, callback)` - Register persistent handler, returns ID
- `niri.events:once(event, callback)` - Register one-time handler
- `niri.events:off(event, handler_id)` - Remove specific handler
- `niri.events:emit(event, data)` - Emit custom events
- `niri.events:list()` - Query registered handlers
- `niri.events:clear(event)` - Remove all handlers for an event

**Integration**: Registered in `runtime.rs` via `init_event_system()` which sets up both legacy API (`niri.on/once/off`) and new proxy API sharing the same handler registry.

### Phase R5: Action System (COMPLETE)

**File: `niri-lua/src/action_proxy.rs`** (~1,544 lines)

Created `ActionProxy` userdata implementing all ~90 compositor actions:
- Uses `ActionCallback` type alias for executing actions
- Helper functions: `parse_size_change()`, `parse_position_change()`, `parse_workspace_reference()`, `parse_layout_switch_target()`
- All actions use **colon syntax**: `niri.action:spawn({"kitty"})`

**Key action categories**:
- System: `quit()`, `spawn()`, `spawn_sh()`, `power_off_monitors()`, `load_config_file()`
- Screenshots: `screenshot()`, `screenshot_screen()`, `screenshot_window()`
- Focus: `focus_column_left/right/first/last()`, `focus_window_up/down()`
- Movement: `move_column_left/right()`, `move_window_up/down()`
- Workspaces: `focus_workspace()`, `move_window_to_workspace()`, `set_workspace_name()`
- Size: `set_window_width()`, `set_column_width()` with string parsing ("+10", "-5%", "50%")
- Floating: `toggle_window_floating()`, `move_floating_window()`
- Overview: `toggle_overview()`, `open_overview()`, `close_overview()`

**Integration**: `LuaRuntime::register_action_proxy(callback)` in `runtime.rs`

**Tests**: 18 unit tests in `action_proxy.rs`, 11 integration tests in `repl_integration.rs`

---

## Current API Summary (After Phase R5)

### Namespaces

| Namespace | Purpose | Example |
|-----------|---------|---------|
| `niri.state` | Query compositor state | `niri.state.windows()` |
| `niri.utils` | Logging and utilities | `niri.utils.log("msg")` |
| `niri.config` | Configuration proxy | `niri.config.layout.gaps = 16` |
| `niri.events` | Event system | `niri.events:on("window:open", fn)` |
| `niri.action` | Compositor actions | `niri.action:spawn({"kitty"})` |

### State API (`niri.state.*`)

```lua
niri.state.windows()        -- Get all windows
niri.state.focused_window() -- Get focused window or nil
niri.state.workspaces()     -- Get all workspaces
niri.state.outputs()        -- Get all outputs/monitors
```

### Utils API (`niri.utils.*`)

```lua
niri.utils.log("message")    -- Info level
niri.utils.debug("message")  -- Debug level
niri.utils.warn("message")   -- Warning level
niri.utils.error("message")  -- Error level
niri.utils.spawn({"cmd"})    -- Spawn process
```

### Config Proxy (`niri.config.*`)

```lua
-- Scalar access
niri.config.layout.gaps = 16
local gap = niri.config.layout.gaps

-- Collection CRUD
niri.config.binds:list()
niri.config.binds:add({ { key = "Super+T", action = "spawn", args = {"kitty"} } })
niri.config.binds:remove({ key = "Super+T" })

-- Apply changes
niri.config:apply()
niri.config:auto_apply(true)
```

### Events API (`niri.events:*`)

```lua
-- Register handlers
local id = niri.events:on("window:open", function(data) ... end)
niri.events:once("window:close", function(data) ... end)  -- fires once

-- Remove handlers
niri.events:off("window:open", id)
niri.events:clear("window:open")  -- remove all handlers for event

-- Emit custom events
niri.events:emit("custom:event", { key = "value" })

-- Query handlers
local info = niri.events:list()  -- returns { total = N, events = {...} }
```

### Action API (`niri.action:*`)

```lua
-- System actions
niri.action:quit()
niri.action:quit(true)  -- skip confirmation
niri.action:spawn({"kitty", "-e", "htop"})
niri.action:spawn_sh("echo hello | grep h")
niri.action:power_off_monitors()
niri.action:load_config_file()

-- Screenshots
niri.action:screenshot()
niri.action:screenshot_screen()
niri.action:screenshot_window()

-- Window/Column focus
niri.action:focus_column_left()
niri.action:focus_column_right()
niri.action:focus_window_up()
niri.action:focus_window_down()

-- Window/Column movement
niri.action:move_column_left()
niri.action:move_column_right()
niri.action:move_window_up()
niri.action:move_window_down()

-- Workspaces
niri.action:focus_workspace(2)           -- by index
niri.action:focus_workspace("main")      -- by name
niri.action:move_window_to_workspace(3)
niri.action:set_workspace_name(1, "dev")

-- Size controls (supports: fixed, %, +N, -N, +N%, -N%)
niri.action:set_window_width(500)
niri.action:set_window_width("50%")
niri.action:set_window_width("+10")
niri.action:set_column_width("-5%")

-- Floating windows
niri.action:toggle_window_floating()
niri.action:move_floating_window("+10", "-5%")
niri.action:focus_floating()
niri.action:focus_tiling()

-- Overview
niri.action:toggle_overview()
niri.action:open_overview()
niri.action:close_overview()

-- Layout
niri.action:switch_layout("next")
niri.action:switch_layout("prev")
niri.action:maximize_column()
```

### Backward-Compatible Aliases (Until R7)

```lua
-- These still work but will be removed in Phase R7
niri.log("msg")     -- → niri.utils.log()
niri.debug("msg")   -- → niri.utils.debug()
niri.on(...)        -- → niri.events:on()
niri.off(...)       -- → niri.events:off()
```

---

## Phase R1-R3 Implementation Summary

### Phase R1: Core Infrastructure (COMPLETE)

**File: `niri-lua/src/config_proxy.rs`** (~680 lines)

- `PendingConfigChanges` - Stores staged configuration changes as JSON
- `ConfigSectionProxy` - Enables `niri.config.layout.gaps = 16` syntax
- `ConfigCollectionProxy` - CRUD operations for binds, window_rules, etc.
- `ConfigProxy` - Main `niri.config` table with `:apply()` method

### Phase R2: IPC Integration (COMPLETE)

**File: `src/niri.rs`** (new methods)

- `State::apply_pending_lua_config()` - Main entry point for applying Lua config changes
- `State::apply_layout_change()` - Helper for layout config changes
- `State::apply_animation_change()` - Helper for animation config changes

**File: `src/ipc/server.rs`** (modified)

- Modified `ExecuteLua` handler to call `apply_pending_lua_config()` after Lua execution

### Phase R3: Namespace Migration (COMPLETE)

**File: `niri-lua/src/runtime_api.rs`**

- Changed `niri.runtime` → `niri.state`
- Renamed: `get_windows()` → `windows()`, `get_focused_window()` → `focused_window()`, etc.

**File: `niri-lua/src/niri_api.rs`**

- Created `niri.utils` namespace for logging and utilities
- Kept top-level aliases for backward compatibility

---

## Key Documentation References

| Document | Purpose |
|----------|---------|
| `docs/LUA_IMPLEMENTATION_ROADMAP.md` | Full roadmap, API design decisions |
| `docs/LUA_GUIDE.md` | User-facing Lua configuration guide |
| `docs/LUA_RUNTIME_STATE_API.md` | Runtime state API documentation |
| `docs/LUA_EVENT_HOOKS.md` | Event system documentation |
| `docs/LUA_CONFIG_STATUS.md` | Implementation status (868 lines) |
| `niri-lua/AGENTS.md` | Crate architecture overview |

---

## Commands Reference

```bash
# Build niri-lua crate
cargo build -p niri-lua

# Run all niri-lua tests
cargo test -p niri-lua

# Run only unit tests
cargo test -p niri-lua --lib

# Run only integration tests
cargo test -p niri-lua --test repl_integration

# Run specific test
cargo test -p niri-lua runtime_api

# Build entire project
cargo build

# Test Lua via IPC REPL (requires running niri)
niri msg action lua "niri.state.windows()"
niri msg action lua "niri.config.layout.gaps = 20; niri.config:apply()"
```

---

## Code Patterns to Follow

### Adding UserData (Lua-accessible types)

```rust
impl UserData for MyType {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("my_method", |lua, this, args: (String,)| {
            // Implementation
            Ok(())
        });
    }
    
    fn add_fields<F: UserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("my_field", |lua, this| {
            Ok(this.value.clone())
        });
    }
}
```

### Thread-Safe State Sharing

```rust
pub type SharedState = Arc<Mutex<MyState>>;

// In Lua registration
let state_clone = state.clone();
methods.add_method("do_thing", move |lua, this, ()| {
    let mut guard = state_clone.lock();
    guard.modify();
    Ok(())
});
```

### Error Handling

```rust
// Return Lua errors
Err(LuaError::external("error message"))

// Or with context
Err(LuaError::external(format!("failed to parse '{}': {}", input, err)))
```

---

## Contact Points in Code

### Where Lua Runtime Lives
- `src/niri.rs` - `Niri` struct has `lua_runtime: Option<LuaRuntime>`
- Initialized during config loading

### Where IPC Requests are Handled
- `src/ipc/server.rs` - `handle_request()` function
- `ExecuteLua` variant handles Lua code execution

### Where Config is Applied
- `src/niri.rs` - `reload_config()` method shows full config reload flow
- `apply_pending_lua_config()` for runtime changes

### Where Events are Emitted
- `src/lua_event_hooks.rs` - Helper functions for emitting events
- Various compositor files call these helpers (see `docs/LUA_FILES_CHECKLIST.md`)

---

## Session History

| Date | Phase | Summary |
|------|-------|---------|
| - | R1 | Created `config_proxy.rs` with proxy tables and pending changes |
| - | R2 | Added `apply_pending_lua_config()`, IPC integration |
| - | R3 | Migrated `niri.runtime` → `niri.state`, created `niri.utils` |
| - | R3+ | Consolidated tests (22 → 11), updated docs to new API |
| - | R4 | Created `events_proxy.rs` with `niri.events:on/once/off/emit/list/clear` methods |
| - | R5 | Created `action_proxy.rs` with ~90 compositor actions via `niri.action:*` |
| - | R6 | Added 6 new events: `window:title_changed`, `window:app_id_changed`, `window:fullscreen`, `config:reload`, `overview:open`, `overview:close` |
| - | R7 | Removed deprecated APIs (`niri.log`, `niri.on`, etc.), updated docs |
| - | R8 | Added Lua config file detection in `niri-config/src/lib.rs` |
| - | R9-R11 | Implemented reactive config system with `apply_pending_lua_config()` (~4000 lines), supporting all scalar sections, collections (binds, window_rules, outputs, spawn_at_startup, environment, workspaces), and startup spawn capture |

**Current test counts:** 442 unit + 105 integration = 547 total tests passing

**Reactive API test results:**
- 47 scalar changes, 117 collection additions applied successfully
- 111 keybindings, 3 spawn_at_startup, 3 outputs working
- XKB options applied, event handlers functional

**Next action:** Phase R12 - Remove obsolete old global-variable API code
