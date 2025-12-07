# Niri Lua Implementation Roadmap

## Status Summary

| Phase | Status | Description |
|-------|--------|-------------|
| Tier 1: Module System | ✅ COMPLETE | Module loader, plugin discovery, event emitter, hot reload |
| Tier 2: Configuration API | ✅ COMPLETE | Full config API, Lua types, validators, extractors |
| Tier 3: Runtime State | ✅ COMPLETE | 4 query functions (windows, focused_window, workspaces, outputs) |
| Tier 4: Event System | ✅ MOSTLY COMPLETE | Core events wired (window, workspace, monitor, overview, config) |
| API Refactor R1-R13 | ✅ COMPLETE | Reactive config proxy, `niri.state/action/events/utils` namespaces |
| Config Side Effects | ✅ COMPLETE | Cursor, keyboard, libinput settings properly applied |
| Async/Safety | 🚧 PLANNED | No execution timeouts yet (see LUA_ASYNC_IMPLEMENTATION.md) |
| Tier 5: Plugin Ecosystem | 🚧 NOT IMPLEMENTED | Basic discovery only; lifecycle/sandbox/IPC pending |
| Tier 6: Developer Experience | ⚙️ PARTIAL | REPL/docs done; type definitions/LSP pending |

---

## Architecture TODOs

> **TODO: Simplify config_proxy.rs** - Uses `serde_json::Value` as intermediary format.
> Evaluate whether direct Lua-to-Config conversion would be more efficient.

> ~~**TODO: Unify event_emitter.rs**~~ - ✅ COMPLETED: Removed unused Rust `EventEmitter` struct,
> kept Lua-based implementation via global tables. File reduced from ~270 to ~240 lines.

## Code Quality Issues

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

**TODO:**
- EmmyLua type definitions for lua_ls autocomplete
- LSP configuration for Neovim/VS Code
- Plugin testing framework
- Additional example plugins

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
