# Niri Lua Implementation Roadmap

## Status Summary

| Phase | Status | Description |
|-------|--------|-------------|
| Tier 1: Module System | ✅ COMPLETE | Module loader, plugin discovery, event emitter |
| Tier 2: Configuration API | ✅ COMPLETE | Full config API, Lua types, validators, extractors |
| Tier 3: Runtime State | ✅ COMPLETE | 4 query functions (windows, focused_window, workspaces, outputs) |
| Tier 4: Event System | ✅ MOSTLY COMPLETE | Core events wired (window, workspace, monitor, overview, config) |
| API Refactor R1-R13 | ✅ COMPLETE | Reactive config proxy, `niri.state/action/events/utils` namespaces |
| Config Side Effects | ✅ COMPLETE | Cursor, keyboard, libinput settings properly applied |
| Async/Safety | ✅ COMPLETE | Timeouts, scheduling, timer API (see LUA_ASYNC_IMPLEMENTATION.md) |
| Tier 5: Plugin Ecosystem | 🚧 NOT IMPLEMENTED | Basic discovery only; lifecycle/sandbox/IPC pending |
| Tier 6: Developer Experience | ⚙️ IN PROGRESS | REPL/docs done; type gen infrastructure complete |
| Reactive State Subscription | 📋 OPTIONAL (Low Priority) | `niri.state.watch()` API for property change callbacks |

---

## Architecture TODOs

> **TODO: Simplify config_proxy.rs** - Uses `serde_json::Value` as intermediary format.
> Evaluate whether direct Lua-to-Config conversion would be more efficient.

> ~~**TODO: Unify event_emitter.rs**~~ - ✅ COMPLETED: Removed unused Rust `EventEmitter` struct,
> kept Lua-based implementation via global tables. File reduced from ~270 to ~240 lines.

## Code Quality Issues

> **BUG: Mutex lock not unwrapped in apply_pending_lua_config()** - `src/niri.rs:1766`
> calls `.has_changes()` on `Result<MutexGuard, PoisonError>` instead of the unwrapped guard.
> Fix: Change `pending_ref.lock()` to `pending_ref.lock().unwrap()`.
> See [LUA_ASYNC_IMPLEMENTATION.md](LUA_ASYNC_IMPLEMENTATION.md#known-issues-and-testing-gaps) for details.

> ~~**TODO: Replace unsafe code in runtime.rs:300-306**~~ - ✅ COMPLETED: Replaced raw pointer
> with `Rc<RefCell<Vec<String>>>` for safe interior mutability.

> ~~**TODO: Add logging for silent .ok()? patterns**~~ - ✅ COMPLETED: Added `trace!` logging
> to 14 locations in config_converter.rs where parse errors were silently swallowed.

> ~~**TODO: Handle channel send failures in runtime_api.rs**~~ - ✅ COMPLETED: Added `log::warn!`
> on channel send failures at lines 214, 242, 270, 300.

> ~~**TODO: Complete extractors.rs**~~ - ✅ CLARIFIED: The extractors for Input, Layout, Output,
> and WindowRule are already implemented in `config_converter.rs` using JSON as an intermediary.
> The `extractors.rs` module contains basic utility functions but is currently unused (dead code).
> Consider removing or integrating it in a future cleanup.

> ~~**TODO: Add doc comments**~~ - ✅ VERIFIED COMPLETE: Both `config_proxy.rs` and `validators.rs`
> have comprehensive documentation including module-level docs, struct/enum docs, field docs,
> and method docs with valid ranges.

> ~~**TODO: Register live action callback for IPC execution**~~ - ✅ COMPLETED: Added calloop
> channel in `main.rs` to pipe Lua actions to `state.do_action()`. Actions executed via
> `niri msg lua` now work correctly (e.g., `niri.action:spawn_sh("kitty")`).

---

## Remaining Work

### Config Converter: Missing Actions

The `action_proxy.rs` has all ~130 actions mapped for `niri.action:*` calls. The `config_converter.rs` now also has comprehensive mappings for parsing bind configurations from Lua (`niri.config.binds`):

**COMPLETED:**
- ✅ `toggle-keyboard-shortcuts-inhibit` - Toggle keyboard shortcuts inhibitor
- ✅ `expand-column-to-available-width` - Expand column to fill available space
- ✅ `center-visible-columns` - Center the visible columns on screen
- ✅ `switch-focus-between-floating-and-tiling` - Switch focus between floating and tiled windows
- ✅ `toggle-column-tabbed-display` - Toggle tabbed display mode for column
- ✅ Added 60+ additional action mappings including focus, move, monitor, floating, and debug actions

### Config Application: Missing Side Effects

When config values are changed via IPC Lua, some require side effects beyond just updating the value. The main refresh loop (`refresh_and_flush_clients`) handles most cases automatically.

**COMPLETED - All Config Side Effects Now Handled:**

The `apply_pending_lua_config()` function in `src/niri.rs` now properly applies side effects when config values change via Lua:

- ✅ `layout.*` - `layout.update_config()` called when layout changes
- ✅ `animations.*` - Clock rate/complete_instantly updated when animations change
- ✅ `cursor.xcursor_theme` / `cursor.xcursor_size` - `cursor_manager.reload()` and `cursor_texture_cache.clear()` called
- ✅ `input.keyboard.xkb.*` - `set_xkb_config()` called for layout/options changes
- ✅ `input.keyboard.repeat_rate` / `repeat_delay` - `keyboard.change_repeat_info()` called
- ✅ `input.touchpad.*` / `input.mouse.*` - `apply_libinput_settings()` called on all devices

**Note:** Output reconfiguration is not yet handled - outputs need explicit reconfiguration via actions.

### Event System: Wiring Status

**Currently wired events:**

| Category | Event | Status |
|----------|-------|--------|
| Lifecycle | `startup` | ✅ Wired (main.rs) |
| Lifecycle | `shutdown` | ✅ Wired (main.rs) |
| Window | `window:open` | ✅ Wired with real data (id, title, app_id) |
| Window | `window:close` | ✅ Wired with real data (id, title, app_id) |
| Window | `window:focus` | ✅ Wired with real data (niri.rs:focus_window) |
| Window | `window:blur` | ✅ Wired with real data (niri.rs:focus_window) |
| Window | `window:title_changed` | ✅ Wired (xdg_shell.rs) |
| Window | `window:fullscreen` | ✅ Wired (xdg_shell.rs) |
| Workspace | `workspace:activate` | ✅ Wired |
| Workspace | `workspace:deactivate` | ✅ Wired |
| Workspace | `workspace:create` | ✅ Wired (ext_workspace.rs) |
| Workspace | `workspace:destroy` | ✅ Wired (ext_workspace.rs) |
| Workspace | `workspace:rename` | ✅ Wired (input/mod.rs) |
| Monitor | `monitor:connect` | ✅ Wired (backend/tty.rs) |
| Monitor | `monitor:disconnect` | ✅ Wired (backend/tty.rs) |
| Output | `output:mode_change` | ✅ Wired (niri.rs:output_resized) |
| Overview | `overview:open` | ✅ Wired (input/mod.rs) |
| Overview | `overview:close` | ✅ Wired (input/mod.rs) |
| Config | `config:reload` | ✅ Wired (niri.rs) |
| Layout | `layout:window_added` | ✅ Wired (compositor.rs) |
| Layout | `layout:window_removed` | ✅ Wired (xdg_shell.rs) |
| Layout | `layout:mode_changed` | ✅ Wired (input/mod.rs) |
| Lock | `lock:activate` | ✅ Wired (niri.rs) |
| Lock | `lock:deactivate` | ✅ Wired (niri.rs) |
| Window | `window:app_id_changed` | ✅ Wired (xdg_shell.rs) |
| Window | `window:move` | ✅ Wired (input/mod.rs - MoveWindowToWorkspaceUp/Down) |
| Window | `window:resize` | ✅ Wired (resize_grab.rs, touch_resize_grab.rs) |
| Window | `window:maximize` | ✅ Wired (xdg_shell.rs - maximize/unmaximize_request) |

**Not supported (by design):**

| Category | Events | Rationale |
|----------|--------|-----------|
| Idle | `idle:start`, `idle:end` | Not exposed via IPC. Smithay's IdleNotifierState doesn't provide Rust callbacks. Idle behavior is better controlled via configuration (timeouts, inhibitors). |
| Keyboard | `key:press`, `key:release` | Not exposed via IPC. Raw key events are extremely noisy (every keystroke), have security concerns (keylogging potential), and are not needed - keybindings cover the use cases. AwesomeWM also does not expose raw key events, using a keybinding registration model instead. |

### Tier 5: Plugin Ecosystem

See [LUA_TIER5_SPEC.md](LUA_TIER5_SPEC.md) for details.

**Status:** 🚧 NOT IMPLEMENTED (discovery only)

**Current state:**
- ✅ Plugin discovery in `~/.config/niri/plugins/`
- ✅ Plugin metadata parsing
- 🚧 Sandbox is a stub - `create_plugin_env()` just copies all globals without restrictions

**TODO:**
- Plugin lifecycle management (enable/disable, on_load/on_unload hooks)
- Plugin sandbox with capability-based permissions
- Dependency resolution with version constraints
- IPC commands: `niri msg plugin list/enable/disable/info`

### Tier 6: Developer Experience

See [LUA_TIER6_SPEC.md](LUA_TIER6_SPEC.md) for details.

**Status:** ⚙️ IN PROGRESS

**Completed:**
- ✅ Interactive REPL (`niri msg lua`)
- ✅ Documentation (LUA_GUIDE.md, LUA_QUICKSTART.md, etc.)
- ✅ Example scripts (10 in `examples/`)
- ✅ Type generation infrastructure (Option C: Custom Registry)
  - `lua_api_schema.rs` - Schema type definitions
  - `api_registry.rs` - Full API registry (~100 actions, 5 UserData types)
  - `build.rs` - EmmyLua generator

**TODO:**
- [ ] Add new modules to `lib.rs`
- [ ] Build and verify `types/api.lua` generation
- [ ] LSP configuration for Neovim/VS Code
- [ ] Plugin testing framework
- [ ] Additional example plugins

**Design Decision:** Using EmmyLua annotations (`---@class`, `---@param`) for compatibility with emmylua-analyzer-rust, lua_ls, and other common LSPs. Not using Luau native types since user's LSP doesn't support them.

---

## Architecture Overview

```
niri-lua/src/
├── lib.rs                  # Module exports
├── runtime.rs              # Lua VM management
├── config.rs               # Configuration loading
├── config_proxy.rs         # Reactive config system (niri.config.*)
├── config_converter.rs     # Apply pending changes to Config
├── action_proxy.rs         # ~90 compositor actions (niri.action:*)
├── events_proxy.rs         # Event subscription (niri.events:on/off)
├── runtime_api.rs          # State queries (niri.state.*)
├── niri_api.rs             # Core API (niri.utils.*)
├── ipc_repl.rs             # IPC Lua execution
├── module_loader.rs        # require() implementation
├── plugin_system.rs        # Plugin discovery
├── hot_reload.rs           # File watching
└── ...

src/
├── lua_event_hooks.rs      # Event emission from compositor
└── ...
```

## API Namespaces

| Namespace | Purpose | Example |
|-----------|---------|---------|
| `niri.config` | Configuration proxy | `niri.config.layout.gaps = 16` |
| `niri.action` | Compositor actions | `niri.action:spawn({"kitty"})` |
| `niri.events` | Event system | `niri.events:on("window:open", fn)` |
| `niri.state` | Query compositor state | `niri.state.windows()` |
| `niri.utils` | Logging and utilities | `niri.utils.log("msg")` |

---

## References

- [LUA_GUIDE.md](LUA_GUIDE.md) - User guide
- [LUA_QUICKSTART.md](LUA_QUICKSTART.md) - Quick start

---

## Optional Feature: Reactive State Subscription

**Status:** 📋 LOW PRIORITY - Not implemented

**Summary:** Enables Lua scripts to receive automatic callbacks when compositor state changes, without polling or relying on manually-emitted discrete events.

### Motivation

The current Lua API has two patterns:

| Pattern | API | Characteristics |
|---------|-----|-----------------|
| **Events (Push)** | `niri.events:on("window:open", fn)` | Discrete occurrences, manually emitted |
| **State (Pull)** | `niri.state.windows()` | Point-in-time snapshots, requires polling |

**Gap:** No way to say "call me when X property changes" without:
- Polling `niri.state.*` functions repeatedly
- Hoping a discrete event exists for that specific change

### Proposed API: `niri.state.watch()`

**Signature:**
```lua
---@param path_or_selector string|fun(state: table): any
---@param callback fun(value: any)
---@param opts? { immediate?: boolean, equals?: fun(a: any, b: any): boolean }
---@return fun(): boolean  -- unwatch function, returns true if successfully unsubscribed
function niri.state.watch(path_or_selector, callback, opts) end
```

**Basic Usage:**
```lua
-- Path-based subscription (simple cases)
local unwatch = niri.state.watch("focused_window", function(win)
    if win then
        print("Now focused: " .. win.title)
    end
end)

-- Watch collections
niri.state.watch("windows", function(windows)
    print("Window count: " .. #windows)
end)

-- Selector-based subscription (complex derived state)
niri.state.watch(function(state)
    local firefox = {}
    for _, w in ipairs(state.windows) do
        if w.app_id == "firefox" then
            table.insert(firefox, w)
        end
    end
    return firefox
end, function(firefox_windows)
    print("Firefox windows: " .. #firefox_windows)
end)

-- Unsubscribe when done
local was_subscribed = unwatch()  -- returns true if was still subscribed
```

**Options:**
```lua
-- Get immediate callback with current value, then on changes
niri.state.watch("windows", callback, { immediate = true })

-- Custom equality for selectors (avoid deep comparison)
niri.state.watch(function(state)
    return state.focused_window
end, callback, {
    equals = function(a, b)
        return (a and a.id) == (b and b.id)
    end
})
```

**Path Constants (for type safety / LSP autocomplete):**
```lua
-- Optional: use constants instead of string literals
niri.state.watch(niri.state.WINDOWS, callback)
niri.state.watch(niri.state.FOCUSED_WINDOW, callback)
niri.state.watch(niri.state.WORKSPACES, callback)
niri.state.watch(niri.state.OUTPUTS, callback)
niri.state.watch(niri.state.FOCUSED_OUTPUT, callback)
```

### Watchable Paths

| Path | Type | Description |
|------|------|-------------|
| `windows` | `Window[]` | All windows |
| `workspaces` | `Workspace[]` | All workspaces |
| `outputs` | `Output[]` | All outputs |
| `focused_window` | `Window?` | Currently focused window (or nil) |
| `focused_output` | `Output?` | Currently focused output |

### Behavioral Guarantees

**1. Batching (Once Per Frame):**
- Callbacks are fired **at most once per event loop iteration**, not per individual state change
- If 5 windows close in one frame, the `windows` callback fires once with the final state
- This matches niri's existing `State::refresh()` pattern and prevents callback spam

**2. Error Handling:**
- If a selector function throws an error, the error is logged and that subscription is **skipped for this cycle**
- Subscriptions are **not permanently disabled** on error - they will be retried next cycle
- If a callback throws, the error is logged but other subscriptions continue processing

**3. Ordering:**
- Path-based subscriptions are processed before selector-based subscriptions
- Within each category, subscriptions fire in registration order
- The `immediate` option fires synchronously during `watch()` call, before returning

**4. Lifecycle:**
- Subscriptions are automatically cleaned up when the Lua runtime is destroyed
- Calling `unwatch()` multiple times is safe (returns `false` on subsequent calls)
- Watching the same path multiple times creates independent subscriptions

### Implementation Spec

#### 1. Subscription Registry (`niri-lua/src/state_watch.rs`)

```rust
pub struct StateWatcher {
    /// Path-based subscriptions: path -> list of subscriptions
    path_subscriptions: HashMap<String, Vec<PathSubscription>>,
    
    /// Selector-based subscriptions: id -> subscription data
    selector_subscriptions: HashMap<u64, SelectorSubscription>,
    
    /// Previous values for path-based change detection
    previous_values: HashMap<String, LuaValue>,
    
    /// Counter for generating unique subscription IDs
    next_id: AtomicU64,
}

struct PathSubscription {
    id: u64,
    callback: RegistryKey,
}

struct SelectorSubscription {
    selector: RegistryKey,           // Lua function(state) -> value
    callback: RegistryKey,           // Lua function(value)
    last_value: Option<LuaValue>,    // For change detection
    equals: Option<RegistryKey>,     // Optional custom comparator
}
```

#### 2. Path Constants

Expose as fields on `niri.state` for LSP support:

```rust
// In runtime_api.rs when setting up niri.state
state_table.set("WINDOWS", "windows")?;
state_table.set("WORKSPACES", "workspaces")?;
state_table.set("OUTPUTS", "outputs")?;
state_table.set("FOCUSED_WINDOW", "focused_window")?;
state_table.set("FOCUSED_OUTPUT", "focused_output")?;
```

#### 2. State Snapshot Object

Reuse existing `StateSnapshot` but expose as Lua table for selectors:

```rust
fn create_state_table(lua: &Lua, snapshot: &StateSnapshot) -> LuaResult<LuaTable> {
    let t = lua.create_table()?;
    t.set("windows", snapshot_to_windows(lua, snapshot)?)?;
    t.set("workspaces", snapshot_to_workspaces(lua, snapshot)?)?;
    t.set("outputs", snapshot_to_outputs(lua, snapshot)?)?;
    t.set("focused_window", snapshot_to_focused_window(lua, snapshot)?)?;
    t.set("focused_output", snapshot_to_focused_output(lua, snapshot)?)?;
    Ok(t)
}
```

#### 3. Change Detection Hook

After state mutations, check subscriptions:

```rust
// In State::refresh() or after specific state changes
fn check_state_subscriptions(&mut self) {
    let Some(runtime) = &self.niri.lua_runtime else { return };
    let Some(watcher) = &runtime.state_watcher else { return };
    
    let snapshot = create_state_snapshot(&self.niri);
    watcher.check_and_notify(&snapshot);
}
```

#### 4. Notification Logic (with Error Handling)

```rust
impl StateWatcher {
    fn check_and_notify(&mut self, lua: &Lua, snapshot: &StateSnapshot) {
        let state_table = match create_state_table(lua, snapshot) {
            Ok(t) => t,
            Err(e) => {
                warn!("Failed to create state table: {}", e);
                return;
            }
        };
        
        // Path-based: compare current value at path with previous
        for (path, subs) in &self.path_subscriptions {
            let current = self.get_path_value(&state_table, path);
            let previous = self.previous_values.get(path);
            
            if !values_equal(previous, &current) {
                for sub in subs {
                    if let Err(e) = self.invoke_callback(lua, &sub.callback, &current) {
                        warn!("Path subscription callback error for '{}': {}", path, e);
                        // Continue processing other subscriptions
                    }
                }
                self.previous_values.insert(path.clone(), current);
            }
        }
        
        // Selector-based: re-run selector, compare result
        for (id, sub) in &mut self.selector_subscriptions {
            // Run selector with error handling
            let new_value = match self.run_selector(lua, &sub.selector, &state_table) {
                Ok(v) => v,
                Err(e) => {
                    warn!("Selector {} threw error (skipping this cycle): {}", id, e);
                    continue;  // Skip but don't disable
                }
            };
            
            // Compare using custom or default equality
            let changed = match &sub.equals {
                Some(eq_fn) => !self.custom_equals(lua, eq_fn, &sub.last_value, &new_value),
                None => !lua_deep_equal(&sub.last_value, &new_value),
            };
            
            if changed {
                if let Err(e) = self.invoke_callback(lua, &sub.callback, &new_value) {
                    warn!("Selector {} callback error: {}", id, e);
                }
                sub.last_value = Some(new_value);
            }
        }
    }
}
```

#### 5. Equality Comparison

**Path-based (structured data):**
- Compare by ID for objects (windows, workspaces, outputs)
- For collections: compare length + set of IDs (order-independent)
- Efficient: no deep table comparison needed

**Selector-based (arbitrary Lua values):**
- Default: deep equality comparison (recursive table comparison)
- Custom: user-provided `equals` function via options
- Recommendation: always provide `equals` for complex selectors to avoid performance issues

```lua
-- Example: efficient ID-based comparison
niri.state.watch(function(state)
    return state.focused_window
end, on_focus_change, {
    equals = function(a, b)
        -- Compare by ID only, not full object
        return (a and a.id) == (b and b.id)
    end
})
```

### Integration Points

| Location | Change |
|----------|--------|
| `niri-lua/src/lib.rs` | Export `state_watch` module |
| `niri-lua/src/runtime.rs` | Add `state_watcher: Option<StateWatcher>` field |
| `niri-lua/src/runtime_api.rs` | Register `niri.state.watch()` function |
| `src/niri.rs` | Call `check_state_subscriptions()` in refresh loop |

### Performance Considerations

1. **Debouncing:** Don't check every frame; batch checks after state mutations settle
2. **Path optimization:** Only re-evaluate paths that could have changed
3. **Selector cost:** Document that selectors should be cheap; they run on every check
4. **Memory:** Store previous values efficiently (IDs only for collections)

### Phased Implementation

| Phase | Scope | Effort |
|-------|-------|--------|
| Phase 1 | Path-based for 5 core paths | ~2-3 days |
| Phase 2 | Selector-based subscriptions | ~2 days |
| Phase 3 | Performance optimization | ~1-2 days |

### Why Low Priority

1. **Existing events cover most use cases** - 25+ discrete events already wired
2. **Polling works** - Scripts can use timers + `niri.state.*` queries
3. **Complexity** - State diffing and subscription management adds maintenance burden
4. **Performance risk** - Must be carefully implemented to avoid frame drops
