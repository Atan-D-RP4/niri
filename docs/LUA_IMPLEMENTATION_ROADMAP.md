# Niri Lua Implementation Roadmap

## Status Summary

| Phase | Status | Description |
|-------|--------|-------------|
| Tier 1: Module System | ✅ COMPLETE | Module loader, plugin discovery, event emitter, hot reload |
| Tier 2: Configuration API | ✅ COMPLETE | Full config API, Lua types, validators, extractors |
| Tier 3: Runtime State | ✅ COMPLETE | Window/workspace/output queries, IPC REPL |
| Tier 4: Event System | ✅ COMPLETE | 11 event types, handler registration, compositor integration |
| API Refactor R1-R13 | ✅ COMPLETE | Reactive config proxy, `niri.state/action/events/utils` namespaces |
| Tier 5: Plugin Ecosystem | ⚙️ PARTIAL | Basic infrastructure done; lifecycle/sandbox pending |
| Tier 6: Developer Experience | ⚙️ PARTIAL | REPL/docs done; type definitions/LSP pending |

---

## Remaining Work

### Tier 5: Plugin Ecosystem

See [LUA_TIER5_SPEC.md](LUA_TIER5_SPEC.md) for details.

**TODO:**
- Plugin lifecycle management (enable/disable, on_load/on_unload hooks)
- Plugin sandbox for isolation
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
- [SESSION_API_REFACTOR.md](SESSION_API_REFACTOR.md) - API refactor phases
