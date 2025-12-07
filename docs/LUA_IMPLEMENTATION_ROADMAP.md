# Niri Lua Implementation Roadmap

## Status Summary

| Phase | Status | Description |
|-------|--------|-------------|
| Tier 1: Module System | ✅ COMPLETE | Module loader, plugin discovery, event emitter, hot reload |
| Tier 2: Configuration API | ✅ COMPLETE | Full config API, Lua types, validators, extractors |
| Tier 3: Runtime State | ✅ COMPLETE | 4 query functions (windows, focused_window, workspaces, outputs) |
| Tier 4: Event System | ⚠️ PARTIAL | Infrastructure complete; most events not wired to compositor |
| API Refactor R1-R13 | ✅ COMPLETE | Reactive config proxy, `niri.state/action/events/utils` namespaces |
| Config Side Effects | ✅ COMPLETE | Cursor, keyboard, libinput settings properly applied |
| Async/Safety | 🚧 PLANNED | No execution timeouts yet (see LUA_ASYNC_IMPLEMENTATION.md) |
| Tier 5: Plugin Ecosystem | 🚧 NOT IMPLEMENTED | Basic discovery only; lifecycle/sandbox/IPC pending |
| Tier 6: Developer Experience | ⚙️ PARTIAL | REPL/docs done; type definitions/LSP pending |

---

## Architecture TODOs

> **TODO: Simplify config_proxy.rs** - Uses `serde_json::Value` as intermediary format.
> Evaluate whether direct Lua-to-Config conversion would be more efficient.

> **TODO: Unify event_emitter.rs** - Contains two parallel implementations:
> 1. Rust `EventEmitter` struct (lines 48-178) - currently unused
> 2. Lua-based implementation via global tables (lines 180-306) - actually used
> Evaluate which approach is better and prune the unused code.

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
| Window | `window:open` | ⚠️ Partial (placeholder data) |
| Workspace | `workspace:activate` | ✅ Wired |

**Defined but NOT wired (TODO):**

| Category | Events |
|----------|--------|
| Window | `window:close`, `window:focus`, `window:blur`, `window:title_changed`, `window:app_id_changed`, `window:fullscreen`, `window:move`, `window:resize`, `window:maximize` |
| Workspace | `workspace:deactivate`, `workspace:create`, `workspace:destroy`, `workspace:rename` |
| Monitor | `monitor:connect`, `monitor:disconnect` |
| Output | `output:mode_change` |
| Layout | `layout:mode_changed`, `layout:window_added`, `layout:window_removed` |
| Overview | `overview:open`, `overview:close` |
| Config | `config:reload` |
| Lock | `lock:activate`, `lock:deactivate` |
| Idle | `idle:start`, `idle:end` |
| Keyboard | `key:press`, `key:release` |

**Note:** Event emission helper functions exist in `src/lua_event_hooks.rs`, but need to be called from the appropriate compositor code paths.

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
