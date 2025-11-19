# Niri Lua Implementation Roadmap

## Executive Summary

This document outlines the complete 12-week implementation plan to bring Niri's Lua API to **ecosystem parity with AwesomeWM, Neovim, and Wezterm**. The plan is organized into 6 tiers, each focusing on specific functionality and building upon the previous tier.

**Current Status:** Lua 5.2 with LuaJIT (matching Neovim), basic config/keybinding support
**Target Status:** Full hackable ecosystem with modules, plugins, events, state access, and excellent DX
**Estimated Time:** 12 weeks

---

## Lua Version Decision

**Selected:** Lua 5.2 with LuaJIT
**Rationale:**
- Matches Neovim's setup for consistency and community knowledge
- LuaJIT provides near-native performance (15-40x faster than standard Lua)
- Lua 5.2 features:
  - `_ENV` for true sandboxing
  - `require()` with better module loading
  - Goto statements (useful for optimization)
  - Bit operations (mlua provides via bit library)
  - Table serialization improvements
- mlua 0.11.4 with `luajit` feature and vendored build
- No external Lua installation required (vendored)

---

## Architecture Overview

```
niri/
├── src/lua_extensions/
│   ├── mod.rs                     # Module root, LuaComponent trait
│   ├── runtime.rs                 # Lua VM management
│   ├── niri_api.rs                # Core Niri API (logging, version)
│   ├── config.rs                  # Configuration loading
│   ├── config_converter.rs        # Config/keybinding conversion
│   │
│   ├── module_loader.rs           # [Tier 1] Module system
│   ├── plugin_system.rs           # [Tier 1] Plugin discovery/loading
│   ├── event_emitter.rs           # [Tier 1/4] Event infrastructure
│   ├── hot_reload.rs              # [Tier 1] File watching & reload
│   │
│   ├── config_api.rs              # [Tier 2] Configuration API tables
│   ├── lua_types.rs               # [Tier 2] Complex type definitions
│   ├── validators.rs              # [Tier 2] Config validation
│   │
│   ├── window_api.rs              # [Tier 3] Window queries
│   ├── workspace_api.rs           # [Tier 3] Workspace queries
│   ├── monitor_api.rs             # [Tier 3] Monitor queries
│   ├── layout_query_api.rs        # [Tier 3] Layout introspection
│   │
│   ├── event_handlers.rs          # [Tier 4] Handler management
│   ├── event_system.rs            # [Tier 4] Event dispatch
│   │
│   ├── plugin_manager.rs          # [Tier 5] Plugin lifecycle
│   ├── plugin_sandbox.rs          # [Tier 5] Plugin isolation
│   ├── plugin_api.rs              # [Tier 5] Plugin-specific APIs
│   └── plugin_registry.rs         # [Tier 5] Plugin discovery/versioning
│
├── src/handlers/
│   └── lua_event_hooks.rs         # [Tier 4] Integration points in Niri core
│
├── examples/
│   ├── niri.lua                   # Example config
│   ├── plugins/                   # [Tier 5] Example plugins
│   └── automation/                # [Tier 5] Example scripts
│
├── docs/
│   ├── LUA_TIER1_SPEC.md          # Tier 1 detailed spec
│   ├── LUA_TIER2_SPEC.md          # Tier 2 detailed spec
│   ├── LUA_TIER3_SPEC.md          # Tier 3 detailed spec
│   ├── LUA_TIER4_SPEC.md          # Tier 4 detailed spec
│   ├── LUA_TIER5_SPEC.md          # Tier 5 detailed spec
│   ├── LUA_TIER6_SPEC.md          # Tier 6 detailed spec
│   └── LUA_IMPLEMENTATION_GUIDE.md # Implementation how-to
│
├── tools/
│   ├── lua-types/                 # [Tier 6] Luau type definitions
│   ├── lua-lsp/                   # [Tier 6] LSP stubs
│   └── lua-tests/                 # [Tier 6] Testing framework
│
└── test_config.lua                # Test file updated with all examples
```

---

## Timeline & Milestones

### Phase 1: Module System & Plugin Foundation (Weeks 1-2)
**Goal:** Enable modular Lua code and plugin discovery

**Deliverables:**
- Module loader with `require()` support
- Plugin directory structure and metadata
- Basic event emitter
- Hot reload capability
- All tests passing

**Success Criteria:**
- Users can create multi-file plugins
- Plugins load from `~/.config/niri/plugins/`
- Changes to config reload automatically
- 120+ lines of new code

**Files to Create:**
- `src/lua_extensions/module_loader.rs`
- `src/lua_extensions/plugin_system.rs`
- `src/lua_extensions/event_emitter.rs` (foundation)
- `src/lua_extensions/hot_reload.rs`
- `docs/LUA_TIER1_SPEC.md`

---

### Phase 2: Full Configuration API (Weeks 3-4)
**Goal:** Support all Niri settings from Lua

**Deliverables:**
- Configuration modules for all settings
- Complex type support (animations, gestures, etc.)
- Type validation and error messages
- Complete configuration parity with KDL

**Success Criteria:**
- Every Niri setting accessible from Lua
- Type checking catches config errors early
- Migration guide from KDL to Lua
- 200+ lines of new code

**Files to Create:**
- `src/lua_extensions/config_api.rs`
- `src/lua_extensions/lua_types.rs`
- `src/lua_extensions/validators.rs`
- `docs/LUA_TIER2_SPEC.md`

---

### Phase 3: Runtime State Access (Weeks 5-6) ⚙️ **IN PROGRESS**
**Goal:** Query Niri's state from Lua

**Documentation:**
- 📘 [Complete Implementation Guide](PHASE3_IMPLEMENTATION_GUIDE.md) - Step-by-step instructions
- 📋 [Quick Reference Card](PHASE3_QUICK_REFERENCE.md) - API surface and checklist

**Current Implementation Plan:**
We're implementing runtime state access via **Event Loop Message Passing** pattern (same as IPC server uses).

**Architecture:**
```rust
// Pattern: Event loop message passing for safe state access
let (tx, rx) = mpsc::channel();
event_loop.insert_idle(move |state| {
    let result = /* query state */;
    tx.send(result).unwrap();
});
rx.recv()  // Blocks until compositor processes
```

**Implementation Phases:**
1. **Phase 1: Infrastructure** (Tasks 1-3)
   - Add `lua_runtime: Option<LuaRuntime>` to `Niri` struct
   - Keep runtime alive in `main.rs` instead of dropping
   - Verify compositor starts successfully

2. **Phase 2: IPC Bridge** (Tasks 4-6)
   - Create `niri-lua/src/ipc_bridge.rs`
   - Convert `Window`, `Workspace`, `Output` → Lua tables
   - Write conversion tests

3. **Phase 3: Runtime API Core** (Tasks 7-10)
   - Create `niri-lua/src/runtime_api.rs` with generic `RuntimeApi<S>`
   - Implement `get_windows()` and `get_focused_window()`
   - Test message passing mechanism

4. **Phase 4: Integration** (Tasks 11-14)
   - Wire up `register_runtime_api()` in `LuaRuntime`
   - Export modules in `lib.rs`
   - Call from `main.rs` after State creation
   - End-to-end test with live Lua script

5. **Phase 5: API Expansion** (Tasks 15-17)
   - Add workspace queries
   - Add action execution (close, focus, move)
   - Document usage

6. **Phase 6: IPC Lua REPL (Bonus)** (Tasks 18-22)
   - Add `Request::Lua` and `Response::LuaResult` to IPC types
   - Implement Lua code execution handler in IPC server
   - Add `execute_string()` method to `LuaRuntime`
   - Add CLI command `niri msg action lua`
   - Implement interactive REPL mode
   - Document usage, examples, and security considerations

**Current Status:** Planning complete, ready for implementation

**Deliverables:**
- [ ] Window query API (`niri.windows.get_all()`, `niri.windows.get_focused()`)
- [ ] Workspace query API (`niri.workspaces.get_all()`, `niri.workspaces.get_active()`)
- [ ] Monitor/Output query API (`niri.outputs.get_all()`)
- [ ] Layout introspection (basic queries)
- [ ] IPC Bridge conversion layer (Window/Workspace/Output → Lua tables)
- [ ] Runtime API with event loop integration (message passing pattern)
- [ ] **Bonus:** IPC Lua REPL (`niri msg action lua`) for interactive debugging and scripting

**Success Criteria:**
- Scripts can query all open windows/workspaces/monitors ✅ Planned
- Window properties accessible (title, geometry, floating, etc.) ✅ Planned
- State changes reflected in queries ✅ Planned
- Zero unsafe code, no lifetime issues ✅ Architecture confirmed
- Synchronous API from Lua's perspective ✅ Architecture confirmed
- 300+ lines of new code

**Files to Create:**
- `niri-lua/src/ipc_bridge.rs` (~150 lines) - Convert IPC types to Lua tables
- `niri-lua/src/runtime_api.rs` (~250 lines) - Generic RuntimeApi<S> with event loop integration
- `niri-lua/tests/runtime_api_tests.rs` (~100 lines) - Integration tests
- `docs/LUA_TIER3_SPEC.md` - Complete API documentation

**Files to Modify:**
- `src/niri.rs` (+1 line at ~line 200: `pub lua_runtime: Option<niri_lua::LuaRuntime>`)
- `src/main.rs` (+15 lines at ~lines 175-220: keep runtime, call register_runtime_api)
- `niri-lua/src/lib.rs` (+7 lines: export runtime_api and ipc_bridge modules)
- `niri-lua/src/runtime.rs` (+20 lines at ~line 60: add register_runtime_api method)
- `niri-lua/Cargo.toml` (verify calloop dependency exists)

---

### Phase 4: Event Handling System (Weeks 7-8)
**Goal:** React to Niri events from Lua

**Event System Architecture Decision: Custom User Events (Neovim-style)**

We will implement **Option B: Custom User Events** to enable a rich plugin ecosystem from the start.

**Rationale:**
- Compositor plugins need inter-plugin communication (e.g., window rules + layout managers)
- Easier to implement upfront than retrofit later
- Aligns with "extensible compositor" goal
- Minimal additional complexity given event system is already planned
- Matches Neovim's proven pattern for extensibility

**Event Types:**
1. **Built-in System Events:**
   - `window_opened`, `window_closed`, `window_focused`
   - `workspace_changed`, `workspace_created`
   - `output_added`, `output_removed`
   - `layout_changed`, `fullscreen_toggled`
   - `key_pressed`, `mouse_moved`

2. **Custom User Events:**
   - Plugins emit: `niri.emit('PluginName::EventName', data)`
   - Plugins listen: `niri.on('PluginName::EventName', handler)`
   - Enables plugin-to-plugin communication
   - Namespaced to avoid conflicts

**Example Usage:**
```lua
-- Plugin A: Window manager
niri.on('window_opened', function(window)
  if window.app_id == 'firefox' then
    niri.emit('WindowManager::BrowserOpened', { id = window.id })
  end
end)

-- Plugin B: Browser controller
niri.on('WindowManager::BrowserOpened', function(data)
  niri.windows.move_to_workspace(data.id, 'browser')
end)
```

**Deliverables:**
- Comprehensive built-in event types ✅
- Custom event emission/listening API ✅
- Event handler registration with error recovery
- Integration points in Niri core
- Event namespacing and validation
- Handler priority/ordering system

**Success Criteria:**
- Plugins can listen to window/workspace/input events
- Plugins can emit and listen to custom events
- Event namespacing prevents conflicts
- Handlers fire at appropriate times
- Event handler errors don't crash Niri
- 350+ lines of new code (increased due to custom events)

**Files to Create:**
- `niri-lua/src/event_handlers.rs` - Handler registration and management
- `niri-lua/src/event_system.rs` - Event dispatch and custom events
- `src/handlers/lua_event_hooks.rs` - Integration points in Niri core
- `docs/LUA_TIER4_SPEC.md`

---

### Phase 5: Plugin Ecosystem (Weeks 9-10)
**Goal:** Full plugin lifecycle and management

**Deliverables:**
- Plugin manager with dependencies
- Plugin lifecycle hooks
- Plugin state persistence
- Plugin registry and versioning

**Success Criteria:**
- Plugins can enable/disable
- Plugin dependencies resolved correctly
- Plugin state survives reloads
- IPC commands for plugin management
- 300+ lines of new code

**Files to Create:**
- `src/lua_extensions/plugin_manager.rs`
- `src/lua_extensions/plugin_sandbox.rs`
- `src/lua_extensions/plugin_api.rs`
- `src/lua_extensions/plugin_registry.rs`
- `docs/LUA_TIER5_SPEC.md`
- Example plugins

---

### Phase 6: Developer Experience (Weeks 11-12)
**Goal:** Excellent tooling and documentation

**Deliverables:**
- Luau type definitions for LSP
- LSP support
- Comprehensive documentation
- Example gallery
- Testing framework
- Interactive REPL

**Success Criteria:**
- IDE autocomplete works
- All APIs documented
- 10+ example plugins
- Testing framework functional
- 1000+ lines of documentation

**Files to Create:**
- `tools/lua-types/*.d.lua`
- `docs/LUA_TIER6_SPEC.md`
- `docs/LUA_API_REFERENCE.md`
- `docs/LUA_BEST_PRACTICES.md`
- `tools/lua-tests/`
- Example plugins and scripts

---

## Implementation Principles

### 1. Non-Breaking
- All new features are additive
- Existing code continues to work
- Deprecation warnings for breaking changes

### 2. Performance-Focused
- Profile all event paths
- Async event delivery where possible
- Lazy loading of modules
- LuaJIT optimization guidelines followed

### 3. User-Centric
- Clear error messages
- Comprehensive documentation
- Examples for common patterns
- Migration guides for AwesomeWM/Neovim users

### 4. Well-Tested
- Unit tests for all modules
- Integration tests for major features
- Mock State for testing without running Niri
- Performance benchmarks

### 5. Security-Conscious
- Input validation
- Plugin sandboxing investigation
- Permission model
- No eval() style dynamic execution (unless explicitly sandboxed)

---

## Tier Comparison Matrix

| Feature | Tier | Scope | Complexity | LOC |
|---------|------|-------|------------|-----|
| Module loading | 1 | Standard require() | Medium | 150 |
| Plugin discovery | 1 | Directory scanning | Low | 100 |
| Basic events | 1 | Event infrastructure | Medium | 120 |
| Hot reload | 1 | File watcher | Medium | 100 |
| Configuration API | 2 | All settings | High | 400 |
| Type definitions | 2 | Complex types | Medium | 200 |
| State queries | 3 | Window/WS/monitor | High | 350 |
| Event handlers | 4 | Full event system | High | 300 |
| Event integration | 4 | Core hooks | High | 200 |
| Plugin lifecycle | 5 | Complete manager | High | 300 |
| Plugin registry | 5 | Versioning/deps | High | 250 |
| Type definitions | 6 | LSP support | Medium | 300 |
| Documentation | 6 | Complete guide | Low | 2000 |
| Tooling | 6 | REPL, testing | Medium | 200 |

**Total Estimated LOC:** ~4500-5000 lines of new Rust code + 2000+ lines of documentation

---

## Risk Assessment & Mitigation

### High Risk: Performance Impact
**Mitigation:**
- Profile event delivery performance
- Async event handling where possible
- Lazy initialization of Lua components
- No synchronous blocking in hot paths

### High Risk: Plugin Conflicts
**Mitigation:**
- Namespace isolation via environment tables
- Dependency resolution
- Plugin conflict detection
- Clear error messages

### Medium Risk: Lua API Instability
**Mitigation:**
- Semantic versioning
- Deprecation warnings
- Compatibility layer for breaking changes
- Extensive testing

### Medium Risk: User Confusion
**Mitigation:**
- Excellent documentation
- Migration guides
- Many examples
- Clear error messages

### Low Risk: Security Issues
**Mitigation:**
- Input validation
- Sandboxing investigation
- Regular security review
- Minimal C code

---

## Success Metrics

- ✅ All Tier 1 features working
- ✅ 50+ users creating custom plugins
- ✅ Performance impact < 5% on event delivery
- ✅ 1000+ downloads/views of example plugins
- ✅ Community plugin registry with 20+ plugins
- ✅ LSP integration working in VS Code/Neovim
- ✅ Ecosystem parity with AwesomeWM/Neovim achieved

---

## Next Steps

Each tier has a detailed specification document:
- [Tier 1 Specification](LUA_TIER1_SPEC.md)
- [Tier 2 Specification](LUA_TIER2_SPEC.md)
- [Tier 3 Specification](LUA_TIER3_SPEC.md)
- [Tier 4 Specification](LUA_TIER4_SPEC.md)
- [Tier 5 Specification](LUA_TIER5_SPEC.md)
- [Tier 6 Specification](LUA_TIER6_SPEC.md)

Implementation guide: [LUA_IMPLEMENTATION_GUIDE.md](LUA_IMPLEMENTATION_GUIDE.md)

---

## References

- [Neovim Lua API](https://neovim.io/doc/user/lua.html)
- [AwesomeWM Documentation](https://awesomewm.org/doc/)
- [Wezterm Lua Configuration](https://wezfurlong.org/wezterm/config/lua.html)
- [mlua Documentation](https://docs.rs/mlua/)
- [Lua 5.2 Manual](https://www.lua.org/manual/5.2/)
- [LuaJIT 2.1 Documentation](https://luajit.org/index.html)
