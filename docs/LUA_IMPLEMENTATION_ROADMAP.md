# Lua API Implementation Roadmap

**Status:** Planning Complete | **Start Date:** November 15, 2025 | **Estimated Duration:** 12 weeks

This document outlines a comprehensive 12-week plan to implement a full-featured Lua scripting API for Niri, achieving feature parity with established window managers and terminal emulators like AwesomeWM, Neovim, and Wezterm.

## Executive Summary

Niri will gain a powerful Lua ecosystem consisting of:
- **Module system** with plugins and automatic hot reload
- **Full configuration API** for all Niri settings
- **Runtime state queries** for windows, workspaces, and monitors
- **Event-driven architecture** with Niri core integration
- **Plugin ecosystem** with dependency management and registry
- **Type definitions, LSP support, and comprehensive documentation**

**Estimated Implementation:** 4500-5000 lines of Rust + 2000+ lines of documentation

---

## Architecture Overview

### Core Stack
- **Lua Version:** Lua 5.2 with LuaJIT (via mlua 0.11.4 with vendored build)
- **Performance:** 15-40x faster than standard Lua
- **Integration:** Direct IPC with Niri daemon
- **Plugin Isolation:** Environment-based sandboxing (lightweight)

### Module Hierarchy
```
niri (root module)
├── config          → Configuration management
├── state            → Runtime state queries
├── events           → Event system
├── plugins          → Plugin management
├── window           → Window operations
├── workspace        → Workspace operations
├── monitor          → Monitor queries
└── layout           → Layout operations
```

---

## 6-Tier Implementation Strategy

### Tier 1: Foundation (Week 1-2) ✅ Planned
**Focus:** Plugin system infrastructure and event foundation

**Deliverables:**
- Module loader with custom search paths (`~/.config/niri/plugins`, `/usr/share/niri/plugins`)
- Plugin metadata system (name, version, author, dependencies)
- Event emitter with `niri.on()`, `niri.once()`, `niri.off()` API
- Hot reload with change detection and callback cleanup
- Basic plugin lifecycle (load → initialize → run → unload)

**Files to Create:** 4
- `src/lua_extensions/module_loader.rs` - Custom require() implementation
- `src/lua_extensions/plugin_system.rs` - Plugin loading and lifecycle
- `src/lua_extensions/event_emitter.rs` - Event registration and dispatch
- `src/lua_extensions/hot_reload.rs` - File watching and reload logic

**Estimated LOC:** 1200

**Success Criteria:**
- [x] Design module system
- [ ] Implement module loader
- [ ] Implement plugin system
- [ ] Implement event emitter
- [ ] Write comprehensive tests
- [ ] Create Tier 1 example plugin

---

### Tier 2: Configuration API (Week 3-4) ✅ Planned
**Focus:** Full configuration access via Lua

**Deliverables:**
- Configuration read/write API for all settings
- Validators and type checking
- Schema introspection
- Integration with existing KDL config (reading)
- Lua config as alternative to KDL

**Supported Settings:**
- `niri.config.animations` - Animation timing (spring, linear, etc.)
- `niri.config.input` - Keyboard/mouse/touchpad settings
- `niri.config.layout` - Tiling algorithm, gaps, borders
- `niri.config.gestures` - Gesture detection and response
- `niri.config.appearance` - Theme, borders, backgrounds
- `niri.config.outputs` - Monitor configuration
- `niri.config.binds` - Keybindings (read/write)

**Files to Create:** 3
- `src/lua_extensions/config_api.rs` - Configuration getters/setters
- `src/lua_extensions/lua_types.rs` - Type definitions and schemas
- `src/lua_extensions/validators.rs` - Input validation

**Estimated LOC:** 1300

**Success Criteria:**
- [ ] Implement config read API
- [ ] Implement config write API
- [ ] Add validators for all settings
- [ ] Write schema introspection
- [ ] Test with real configs

---

### Tier 3: Runtime State Queries (Week 5-6) ✅ Planned
**Focus:** Read-only access to Niri's runtime state

**Deliverables:**
- Window query API (active, by ID, by workspace, by criteria)
- Workspace query API (active, by name, list all)
- Monitor query API (active, by index, physical properties)
- Layout state query API
- Workspace/window tree introspection

**API Examples:**
```lua
-- Window queries
windows = niri.state.windows()
active_window = niri.state.active_window()
windows_on_workspace = niri.state.windows_on_workspace(workspace_id)

-- Workspace queries
workspaces = niri.state.workspaces()
active_workspace = niri.state.active_workspace()
workspace_by_name = niri.state.workspace_by_name("1")

-- Monitor queries
monitors = niri.state.monitors()
active_monitor = niri.state.active_monitor()
```

**Files to Create:** 4
- `src/lua_extensions/window_api.rs` - Window queries
- `src/lua_extensions/workspace_api.rs` - Workspace queries
- `src/lua_extensions/monitor_api.rs` - Monitor queries
- `src/lua_extensions/layout_query_api.rs` - Layout introspection

**Estimated LOC:** 1100

**Success Criteria:**
- [ ] Implement window query API
- [ ] Implement workspace query API
- [ ] Implement monitor query API
- [ ] Write comprehensive filter examples
- [ ] Performance test with 50+ windows

---

### Tier 4: Event Integration (Week 7-8) ✅ Planned
**Focus:** Deep event integration with Niri core

**Deliverables:**
- Event type definitions (window, workspace, focus, action)
- Handler registration with priority queue
- Integration hooks in Niri core (xdg_shell, compositor, layout)
- Error handling and event ordering
- Event filtering and metadata

**Event Types:**
```lua
niri.on("window:open", function(ev) end)           -- Window opened
niri.on("window:close", function(ev) end)          -- Window closed
niri.on("window:focus", function(ev) end)          -- Window focus changed
niri.on("workspace:enter", function(ev) end)       -- Workspace activated
niri.on("workspace:leave", function(ev) end)       -- Workspace deactivated
niri.on("layout:changed", function(ev) end)        -- Layout mode changed
niri.on("action", function(ev) end)                -- Action executed
niri.on("monitor:connect", function(ev) end)       -- Monitor added
niri.on("monitor:disconnect", function(ev) end)    -- Monitor removed
```

**Niri Core Integration Points:**
- `src/handlers/xdg_shell.rs` - Window events
- `src/handlers/compositor.rs` - Compositor events
- `src/layout/mod.rs` - Layout/workspace events
- `src/backend/mod.rs` - Monitor hotplug events

**Files to Create:** 3
- `src/lua_extensions/event_handlers.rs` - Handler registration
- `src/lua_extensions/event_system.rs` - Event dispatch system
- `src/lua_extensions/lua_event_hooks.rs` - Hooks for Niri core

**Files to Modify:** 4
- `src/handlers/xdg_shell.rs` - Add window event hooks
- `src/handlers/compositor.rs` - Add compositor hooks
- `src/layout/mod.rs` - Add workspace/layout hooks
- `src/backend/mod.rs` - Add monitor hotplug hooks

**Estimated LOC:** 900 (new) + 400 (modified)

**Success Criteria:**
- [ ] Define all event types
- [ ] Implement event dispatch
- [ ] Add Niri core hooks
- [ ] Test event ordering
- [ ] Write event handler examples

---

### Tier 5: Plugin Ecosystem (Week 9-10) ✅ Planned
**Focus:** Full plugin ecosystem with dependency management

**Deliverables:**
- Plugin manager with install/uninstall
- Dependency resolver (declarative dependencies)
- Plugin registry (local + remote)
- Plugin state persistence (JSON in `~/.local/share/niri/plugins/`)
- Lifecycle hooks (init, enable, disable, remove)
- Plugin sandbox improvements

**Plugin Manifest Format:**
```lua
-- plugin.lua or plugin-manifest.toml
{
  name = "example-plugin",
  version = "1.0.0",
  author = "Author Name",
  description = "Does something useful",
  dependencies = {
    "core-utils >= 1.0.0",
  },
  exports = {
    setup = function() end,
    on_init = function() end,
  }
}
```

**Files to Create:** 4
- `src/lua_extensions/plugin_manager.rs` - Plugin install/uninstall
- `src/lua_extensions/plugin_sandbox.rs` - Enhanced sandboxing
- `src/lua_extensions/plugin_api.rs` - Plugin API
- `src/lua_extensions/plugin_registry.rs` - Registry management

**Estimated LOC:** 1200

**Success Criteria:**
- [ ] Implement plugin manager
- [ ] Implement dependency resolver
- [ ] Add state persistence
- [ ] Write registry system
- [ ] Create 5+ example plugins

---

### Tier 6: Developer Experience (Week 11-12) ✅ Planned
**Focus:** LSP, type definitions, documentation, and tooling

**Deliverables:**
- Luau type definitions (`.d.lua` files for IDE support)
- LSP stub generation for Neovim/VS Code
- Comprehensive documentation with examples
- Interactive REPL for testing
- Debugger hooks
- Testing framework for plugins

**Type Definitions:**
```lua
-- niri.d.lua (Luau type stubs)
declare module "niri" do
  function log(msg: string, level?: string)
  namespace config do
    function get_animations(): AnimationConfig
    function set_animations(config: AnimationConfig)
  end
  namespace state do
    function windows(): Window[]
    function active_window(): Window?
  end
  namespace events do
    function on(event: string, callback: function)
    function off(event: string, callback: function)
  end
end
```

**Documentation:**
- `LUA_GUIDE.md` - Comprehensive user guide
- `LUA_QUICKSTART.md` - 5-minute quick start
- `LUA_EMBEDDING.md` - Architecture and integration details
- Example plugins with source code walkthroughs
- Video tutorial outline

**Files to Create:** 4
- `docs/niri.d.lua` - Luau type definitions
- `tools/generate-lsp-stubs.rs` - LSP stub generator
- `examples/plugins/` - 5+ example plugins with full source
- `docs/testing-framework.md` - Plugin testing guide

**Estimated LOC:** 800 (code) + 500+ (examples)

**Success Criteria:**
- [ ] Create type definitions
- [ ] Generate LSP stubs
- [ ] Write comprehensive docs
- [ ] Create 5+ example plugins
- [ ] Write testing framework guide

---

## Timeline

| Week | Tier | Tasks | Estimated LOC |
|------|------|-------|---------------|
| 1-2 | 1 | Module loader, plugins, events, hot reload | 1200 |
| 3-4 | 2 | Config API, validators, schemas | 1300 |
| 5-6 | 3 | Window/workspace/monitor queries | 1100 |
| 7-8 | 4 | Event integration with Niri core | 1300 |
| 9-10 | 5 | Plugin manager, registry, state persistence | 1200 |
| 11-12 | 6 | Type defs, LSP, docs, examples, REPL | 1300 |
| **Total** | **All** | **Full Implementation** | **~7400** |

---

## Risk Assessment

### Technical Risks

| Risk | Impact | Mitigation |
|------|--------|-----------|
| Event ordering race conditions | High | Comprehensive testing, use channels for ordering |
| Plugin sandbox escape | Medium | Regular security audit, use environment tables only |
| Performance regression | Medium | Benchmark event dispatch, lazy-load heavy features |
| Breaking changes to IPC | High | Version Niri API, use semver |
| Lua/LuaJIT compatibility | Low | Use mlua's abstraction layer, test both |

### Operational Risks

| Risk | Impact | Mitigation |
|------|--------|-----------|
| Scope creep | Medium | Stick to spec, defer advanced features |
| Maintainability burden | Medium | Excellent documentation, automated tests |
| Community adoption | Medium | Write great examples, create plugin template |
| Documentation lag | Medium | Write docs as code is written, not after |

---

## Success Metrics

### Quantitative

- **Adoption:** 50+ users creating custom plugins within 6 months
- **Plugin Registry:** 20+ community plugins available
- **Performance:** < 5% impact on event delivery latency
- **Coverage:** 100% API endpoint coverage with tests
- **Documentation:** 1000+ downloads/views of example plugins

### Qualitative

- Plugin ecosystem parity with AwesomeWM/Neovim
- Community feedback indicates API is intuitive
- Plugin developers report rapid development iteration
- No critical security issues in plugin system

---

## Next Steps

### Phase 1: Documentation (Completed)
- ✅ Create tier specifications (5 documents)
- ⏳ Create implementation roadmap
- ⏳ Create comprehensive guides

### Phase 2: Core Implementation (Weeks 1-8)
- [ ] Implement Tiers 1-4 (module system, config, queries, events)
- [ ] Add Niri core integration hooks
- [ ] Write integration tests

### Phase 3: Plugin Ecosystem (Weeks 9-10)
- [ ] Implement plugin manager and registry
- [ ] Write 5+ example plugins
- [ ] Create plugin template

### Phase 4: Polish & Documentation (Weeks 11-12)
- [ ] Add type definitions and LSP support
- [ ] Write comprehensive guides
- [ ] Create video tutorial outline

---

## File Structure

```
/home/atan/Develop/repos/niri/
├── src/lua_extensions/
│   ├── module_loader.rs              # Tier 1
│   ├── plugin_system.rs              # Tier 1
│   ├── event_emitter.rs              # Tier 1
│   ├── hot_reload.rs                 # Tier 1
│   ├── config_api.rs                 # Tier 2
│   ├── lua_types.rs                  # Tier 2
│   ├── validators.rs                 # Tier 2
│   ├── window_api.rs                 # Tier 3
│   ├── workspace_api.rs              # Tier 3
│   ├── monitor_api.rs                # Tier 3
│   ├── layout_query_api.rs           # Tier 3
│   ├── event_handlers.rs             # Tier 4
│   ├── event_system.rs               # Tier 4
│   ├── lua_event_hooks.rs            # Tier 4
│   ├── plugin_manager.rs             # Tier 5
│   ├── plugin_sandbox.rs             # Tier 5
│   ├── plugin_api.rs                 # Tier 5
│   ├── plugin_registry.rs            # Tier 5
│   └── mod.rs                        # Updated
│
├── docs/
│   ├── LUA_TIER1_SPEC.md             # ✅ Complete
│   ├── LUA_TIER2_SPEC.md             # ✅ Complete
│   ├── LUA_TIER3_SPEC.md             # ✅ Complete
│   ├── LUA_TIER4_SPEC.md             # ✅ Complete
│   ├── LUA_TIER5_SPEC.md             # ✅ Complete
│   ├── LUA_TIER6_SPEC.md             # ⏳ Pending
│   ├── LUA_IMPLEMENTATION_ROADMAP.md # ⏳ Creating
│   ├── LUA_GUIDE.md                  # ⏳ Pending
│   ├── LUA_QUICKSTART.md             # ⏳ Pending
│   ├── LUA_EMBEDDING.md              # ⏳ Pending
│   └── niri.d.lua                    # ⏳ Pending (Tier 6)
│
└── examples/
    └── plugins/
        ├── example-statusbar/        # ⏳ Pending
        ├── example-keybind-helper/   # ⏳ Pending
        ├── example-layout-switcher/  # ⏳ Pending
        ├── example-window-matcher/   # ⏳ Pending
        └── example-workspace-tabs/   # ⏳ Pending
```

---

## Definition of Done

A tier is considered complete when:

1. ✅ All specified APIs are implemented
2. ✅ Comprehensive unit and integration tests exist
3. ✅ Documentation is complete with examples
4. ✅ Performance benchmarks are recorded
5. ✅ At least one example/use case is provided
6. ✅ Code review completed (if in team environment)
7. ✅ Changes committed to repository

---

## References

- **LUA_TIER1_SPEC.md** - Foundation layer specification
- **LUA_TIER2_SPEC.md** - Configuration API specification
- **LUA_TIER3_SPEC.md** - State queries specification
- **LUA_TIER4_SPEC.md** - Event system specification
- **LUA_TIER5_SPEC.md** - Plugin ecosystem specification
- **LUA_CONFIG_STATUS.md** - Current Lua implementation status

---

**Document Version:** 1.0  
**Last Updated:** November 15, 2025  
**Author:** OpenCode Assistant
