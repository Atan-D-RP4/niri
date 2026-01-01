# Config API Simplification: Implementation Plan

## Status: Ready for Implementation
## Spec: config-api-simplification-spec.md
## Date: 2024-12

---

## Executive Summary

This document provides the detailed implementation plan for the Config API Simplification project. The goal is to replace ~40 generated proxy structs with a unified dynamic property system using `PropertyRegistry` + single `ConfigProxy` UserData.

### Current State

- **40 LuaConfigProxy structs** in `niri-lua/src/config_proxies.rs`
- **4 derive macros** in `niri-lua-derive`: `LuaConfigProxy`, `LuaEnum`, `DirtyFlags`, `FromLuaTable`
- **Arc<Mutex>** pattern for `ConfigState`, `ConfigWrapper`, `SharedEventHandlers`
- **21 DirtyFlag variants** in `config_dirty.rs`
- **~769 lines** in `config_proxy.rs` derive macro

### Target State

- **1 ConfigProxy struct** with dynamic `__index`/`__newindex`
- **1 PropertyRegistry** with all config paths registered
- **1 ConfigProperties derive macro** (simpler than current)
- **Rc<RefCell>** for all non-cross-thread components
- **Signal emission** on config changes via `config::<path>` events

---

## Phase 1: Infrastructure (Week 1)

### Goals
- Create core data structures for the new system
- No changes to existing functionality yet

### Tasks

#### 1.1 Create PropertyRegistry and PropertyDescriptor
**File:** `niri-lua/src/property_registry.rs` (NEW)

```rust
// Core types to implement:
pub struct PropertyRegistry { properties: BTreeMap<String, PropertyDescriptor> }
pub struct PropertyDescriptor { path, ty, dirty_flag, getter, setter, signal }
pub enum PropertyType { Bool, Integer, Number, String, Enum{..}, Array(..), Nested }
```

**Acceptance Criteria:**
- [ ] `PropertyRegistry::add()` stores descriptors by path
- [ ] `PropertyRegistry::get()` retrieves by path
- [ ] `PropertyRegistry::children()` returns direct child keys for a prefix
- [ ] `PropertyRegistry::global()` returns `&'static PropertyRegistry` via `OnceLock`
- [ ] Unit tests for all methods

#### 1.2 Create ConfigProxy UserData
**File:** `niri-lua/src/config_proxy.rs` (NEW, replaces parts of config_wrapper.rs)

```rust
pub struct ConfigProxy { current_path: String }

impl UserData for ConfigProxy {
    // __index, __newindex, __pairs, snapshot(), __tostring
}
```

**Acceptance Criteria:**
- [ ] `__index` returns child proxy for nested paths, value for leaf paths
- [ ] `__newindex` validates and sets value, marks dirty flag, emits signal
- [ ] `__pairs` iterates direct children
- [ ] `:snapshot()` returns plain Lua table copy
- [ ] Error messages include path and expected type
- [ ] Unit tests for all metamethods

#### 1.3 Implement DirtyFlag Inference
**File:** `niri-lua/src/property_registry.rs`

```rust
pub fn infer_dirty_flag(path: &str) -> DirtyFlag {
    match path.split('.').next().unwrap() {
        "cursor" => DirtyFlag::Cursor,
        "layout" => DirtyFlag::Layout,
        // ... 21 variants
    }
}
```

**Acceptance Criteria:**
- [ ] All 21 prefix→DirtyFlag mappings implemented
- [ ] Unknown prefix defaults to `DirtyFlag::Misc`
- [ ] Unit tests for all prefixes

#### 1.4 Add Helper Functions
**File:** `niri-lua/src/property_registry.rs`

```rust
pub fn parse_enum_variant<T: FromStr>(value: &str, variants: &[&str], name: &str) -> LuaResult<T>
pub fn validate_array_elements<T, F>(lua: &Lua, table: &LuaTable, convert: F) -> LuaResult<Vec<T>>
```

**Acceptance Criteria:**
- [ ] Enum parsing provides clear error: `invalid value "x", expected one of: A, B, C`
- [ ] Array validation reports index: `invalid element at index 2: expected integer`
- [ ] Unit tests for error cases

### Phase 1 Deliverables
- `niri-lua/src/property_registry.rs` (~250 lines)
- `niri-lua/src/config_proxy.rs` (~200 lines)
- Unit tests in `niri-lua/tests/property_registry_tests.rs`
- No changes to existing behavior

---

## Phase 2: Derive Macro (Week 2)

### Goals
- Implement the new `ConfigProperties` derive macro
- Coexists with old macro during transition

### Tasks

#### 2.1 Create ConfigProperties Trait
**File:** `niri-lua-traits/src/lib.rs` (ADD)

```rust
pub trait ConfigProperties {
    fn register(registry: &mut PropertyRegistry);
}
```

**Acceptance Criteria:**
- [ ] Trait defined with single `register` method
- [ ] Exported from crate

#### 2.2 Implement Derive Macro for Structs
**File:** `niri-lua-derive/src/config_properties.rs` (NEW)

Generates registration code for struct fields:

```rust
#[derive(ConfigProperties)]
#[config(prefix = "cursor")]
pub struct CursorConfig {
    pub xcursor_size: u8,
    pub xcursor_theme: String,
    #[config(no_signal)]
    pub hide_after_inactive_ms: Option<u32>,
    #[config(skip)]
    internal_cache: Option<CursorCache>,
}
```

**Supported Types:**
- Primitives: `bool`, `u8-u64`, `i8-i64`, `f32`, `f64`, `String`
- Option<T> for nullable values
- Vec<T> for arrays
- Nested structs (recursive registration)
- Enums (string conversion)

**Acceptance Criteria:**
- [ ] `#[config(prefix = "...")]` attribute required on struct
- [ ] `#[config(dirty = "...")]` optional override
- [ ] `#[config(skip)]` excludes field
- [ ] `#[config(no_signal)]` disables signal for field
- [ ] Generates correct getter/setter closures
- [ ] Handles Option<T> with nil support
- [ ] Integration test with simple struct

#### 2.3 Implement Derive Macro for Enums
**File:** `niri-lua-derive/src/config_properties.rs`

Generates variant list and conversion methods:

```rust
#[derive(ConfigProperties)]
pub enum CenterFocusedColumn {
    Never,
    Always,
    OnOverflow,
}

// Generated:
impl CenterFocusedColumn {
    const VARIANTS: &'static [&'static str] = &["Never", "Always", "OnOverflow"];
    fn to_lua_string(&self) -> &'static str { ... }
    fn from_lua_string(s: &str) -> LuaResult<Self> { ... }
}
```

**Acceptance Criteria:**
- [ ] Generates `VARIANTS` constant
- [ ] Generates `to_lua_string()` method
- [ ] Generates `from_lua_string()` with clear error message
- [ ] Case-sensitive matching
- [ ] Unit tests for conversion

#### 2.4 Handle Nested Structs
**File:** `niri-lua-derive/src/config_properties.rs`

For nested struct fields, generate recursive registration:

```rust
// For LayoutConfig with nested Struts
registry.add("layout.struts", PropertyDescriptor::nested(...));
Struts::register_with_prefix(registry, "layout.struts");
```

**Acceptance Criteria:**
- [ ] Nested structs registered with full path prefix
- [ ] Parent path marked as `PropertyType::Nested`
- [ ] Deep nesting works (3+ levels)
- [ ] Integration test with nested config

#### 2.5 Handle Arrays
**File:** `niri-lua-derive/src/config_properties.rs`

Array fields generate element validation:

```rust
// For Vec<ColumnWidth>
registry.add("layout.preset_column_widths", PropertyDescriptor::new(
    ...,
    PropertyType::Array(Box::new(PropertyType::Nested)),
    |lua, config| { /* convert to Lua table */ },
    |lua, config, value| { /* validate each element */ },
));
```

**Acceptance Criteria:**
- [ ] Arrays of primitives work
- [ ] Arrays of enums work
- [ ] Arrays of structs work (nested conversion)
- [ ] Element-by-element validation with index in error
- [ ] Integration test with array field

### Phase 2 Deliverables
- `niri-lua-derive/src/config_properties.rs` (~500 lines)
- Updates to `niri-lua-derive/src/lib.rs`
- `niri-lua-traits/src/lib.rs` updated
- Integration tests in `niri-lua/tests/config_properties_tests.rs`
- Old macros still work (parallel existence)

---

## Phase 3: Registration (Week 3)

### Goals
- Add `#[derive(ConfigProperties)]` to all config structs
- Build complete registry
- Verify registry completeness

### Tasks

#### 3.1 Annotate Root Config Struct
**File:** `niri-config/src/lib.rs` or `niri-lua/src/config_proxies.rs`

```rust
#[derive(ConfigProperties)]
#[config(prefix = "")]  // Root level
pub struct Config {
    pub cursor: CursorConfig,
    pub layout: LayoutConfig,
    // ... all subsystems
}
```

**Acceptance Criteria:**
- [ ] Root Config marked with empty prefix
- [ ] All top-level fields annotated
- [ ] Compiles without errors

#### 3.2 Annotate All Subsystem Configs (40 structs)

| Subsystem | Structs | File |
|-----------|---------|------|
| Cursor | CursorConfig | cursor.rs |
| Layout | LayoutConfig, FocusRingConfig, BorderConfig, ShadowConfig, TabIndicatorConfig, InsertHintConfig, StrutsConfig | layout.rs |
| Input | InputConfig, KeyboardConfig, XkbConfig, TouchpadConfig, MouseConfig, TrackpointConfig, TrackballConfig, TabletConfig, TouchConfig | input.rs |
| Animations | AnimationsConfig + 9 animation types | animations.rs |
| Overview | OverviewConfig | overview.rs |
| Gestures | GesturesConfig | gestures.rs |
| RecentWindows | RecentWindowsConfig | recent_windows.rs |
| Clipboard | ClipboardConfig | clipboard.rs |
| HotkeyOverlay | HotkeyOverlayConfig | hotkey_overlay.rs |
| ConfigNotification | ConfigNotificationConfig | config_notification.rs |
| Debug | DebugConfig | debug.rs |
| XwaylandSatellite | XwaylandSatelliteConfig | xwayland.rs |

**Acceptance Criteria:**
- [ ] All 40 structs annotated
- [ ] Each struct has correct `prefix`
- [ ] Nested relationships preserved
- [ ] All compiles

#### 3.3 Annotate All Enums

Key enums to annotate:
- `CenterFocusedColumn`
- `ResizeGrabZone`
- `TabPosition`
- Animation easing enums
- Input mode enums

**Acceptance Criteria:**
- [ ] All config enums annotated
- [ ] Variant names exposed as strings
- [ ] Compiles without errors

#### 3.4 Initialize Registry at Startup
**File:** `niri-lua/src/lib.rs` or `niri-lua/src/runtime.rs`

```rust
pub fn init_lua(config: Config) -> Lua {
    // Initialize global registry
    PropertyRegistry::init();
    
    // Verify registry completeness
    debug_assert!(PropertyRegistry::global().contains("cursor.xcursor_size"));
    // ... more assertions
    
    let lua = Lua::new();
    // ...
}
```

**Acceptance Criteria:**
- [ ] Registry initialized once at startup
- [ ] Registry contains all expected paths
- [ ] Debug assertions verify completeness
- [ ] Test that prints all registered paths

#### 3.5 Create Registry Verification Test
**File:** `niri-lua/tests/registry_completeness_test.rs`

```rust
#[test]
fn test_registry_contains_all_config_paths() {
    PropertyRegistry::init();
    let registry = PropertyRegistry::global();
    
    // Core paths
    assert!(registry.contains("cursor.xcursor_size"));
    assert!(registry.contains("layout.gaps"));
    assert!(registry.contains("animations.slowdown"));
    // ... comprehensive list
}
```

**Acceptance Criteria:**
- [ ] Test covers all ~200+ config paths
- [ ] Test fails if path missing
- [ ] Test documents expected API

### Phase 3 Deliverables
- All 40 config structs annotated
- All enums annotated
- Registry initialized at startup
- Completeness test passing
- Old proxy code still active (not removed yet)

---

## Phase 4: Switch to New System (Week 4)

### Goals
- Replace `niri.config` with new ConfigProxy
- Remove old generated proxies
- Verify all Lua scripts work

### Tasks

#### 4.1 Replace niri.config Registration
**File:** `niri-lua/src/config_wrapper.rs`

Before:
```rust
globals.set("config", ConfigWrapper::new(state.clone()))?;
```

After:
```rust
globals.set("config", ConfigProxy { current_path: String::new() })?;
```

**Acceptance Criteria:**
- [ ] `niri.config` returns ConfigProxy (root)
- [ ] All existing Lua scripts work unchanged
- [ ] Error messages clear and helpful

#### 4.2 Wire Signal Emission
**File:** `niri-lua/src/config_proxy.rs`

In `__newindex`:
```rust
if desc.signal {
    if let Some(events) = lua.app_data_ref::<Rc<RefCell<EventSystem>>>() {
        events.borrow().emit(lua, &format!("config::{}", path), /* payload */)?;
    }
}
```

**Acceptance Criteria:**
- [ ] `config::cursor.xcursor_size` event emitted on change
- [ ] Payload includes `path` and `value`
- [ ] `#[config(no_signal)]` fields don't emit
- [ ] Integration test for signal emission

#### 4.3 Update Example Scripts
**Files:** `examples/config_api_*.lua`

Verify all example scripts work with new system:
- `config_api_demo.lua`
- `config_api_dump.lua`
- `config_api_usage.lua`

**Acceptance Criteria:**
- [ ] All examples run without errors
- [ ] Output matches expected behavior
- [ ] Document any API changes

#### 4.4 Remove Old Generated Proxies
**Files to modify/remove:**
- `niri-lua/src/config_proxies.rs` - REMOVE (40 proxy structs)
- `niri-lua/src/config_wrapper.rs` - SIMPLIFY (remove proxy macros)
- `niri-lua-derive/src/config_proxy.rs` - REMOVE (old macro)
- `niri-lua-derive/src/lib.rs` - REMOVE `LuaConfigProxy` export

**Acceptance Criteria:**
- [ ] ~1000+ lines removed
- [ ] No compilation errors
- [ ] All tests pass
- [ ] `cargo clippy` clean

#### 4.5 Update config_api.rs Read-Only API
**File:** `niri-lua/src/config_api.rs`

Determine if read-only table builder is still needed. Options:
1. Remove if ConfigProxy handles all use cases
2. Keep if specific read-only mode needed
3. Simplify using PropertyRegistry

**Acceptance Criteria:**
- [ ] Decision documented
- [ ] Implementation matches decision
- [ ] Tests updated

### Phase 4 Deliverables
- `niri.config` uses ConfigProxy
- Signal emission working
- ~1000+ lines removed
- All example scripts working
- All tests passing

---

## Phase 5: Arc→Rc Migration (Week 5)

### Goals
- Convert non-cross-thread components to Rc<RefCell>
- Improve performance and simplify code

### Tasks

#### 5.1 Migrate ConfigState
**File:** `niri-lua/src/config_state.rs`

Before:
```rust
pub struct ConfigState {
    config: Arc<Mutex<Config>>,
    dirty_flags: Arc<Mutex<ConfigDirtyFlags>>,
}
```

After:
```rust
pub struct ConfigState {
    config: Rc<RefCell<Config>>,
    dirty_flags: Rc<RefCell<ConfigDirtyFlags>>,
}
```

**Acceptance Criteria:**
- [ ] All `lock().unwrap()` → `borrow()`/`borrow_mut()`
- [ ] No deadlock panics
- [ ] Performance equal or better
- [ ] Unit tests pass

#### 5.2 Migrate SharedEventHandlers
**File:** `niri-lua/src/events_proxy.rs`

Before:
```rust
type SharedEventHandlers = Arc<Mutex<EventHandlers>>;
```

After:
```rust
type SharedEventHandlers = Rc<RefCell<EventHandlers>>;
```

**Acceptance Criteria:**
- [ ] All access patterns updated
- [ ] Event emission still works
- [ ] Tests pass

#### 5.3 Migrate StateHandle Fields
**File:** `niri-lua/src/state_handle.rs`

Migrate:
- `outputs: Arc<Mutex<Vec<Output>>>`
- `cursor_position: Arc<Mutex<Option<Point>>>`
- Other shared state fields

**Acceptance Criteria:**
- [ ] All StateHandle fields migrated
- [ ] Access patterns updated
- [ ] No Send/Sync bounds where not needed
- [ ] Tests pass

#### 5.4 Migrate ConfigWrapper
**File:** `niri-lua/src/config_wrapper.rs`

Before:
```rust
pub struct ConfigWrapper {
    state: Arc<Mutex<ConfigState>>,
}
```

After:
```rust
pub struct ConfigWrapper {
    state: Rc<RefCell<ConfigState>>,
}
```

**Acceptance Criteria:**
- [ ] All access patterns updated
- [ ] ConfigProxy integration works
- [ ] Tests pass

#### 5.5 Exception: Keep ProcessManager Arc<Mutex>
**File:** `niri-lua/src/process.rs`

ProcessManager must remain Arc<Mutex> because worker threads for stdout/stderr monitoring access it.

**Acceptance Criteria:**
- [ ] ProcessManager unchanged
- [ ] Documented in code comments
- [ ] No regression in process handling

### Phase 5 Deliverables
- ConfigState, SharedEventHandlers, StateHandle, ConfigWrapper migrated
- ProcessManager exception documented
- All tests passing
- No deadlock panics

---

## Phase 6: Cleanup and Documentation (Week 6)

### Goals
- Remove dead code
- Update documentation
- Final integration tests

### Tasks

#### 6.1 Remove Old Derive Macro Code
**Files:**
- `niri-lua-derive/src/config_proxy.rs` - REMOVE
- `niri-lua-derive/src/attributes.rs` - SIMPLIFY (remove unused)

**Acceptance Criteria:**
- [ ] ~800 lines removed
- [ ] Only `ConfigProperties` macro remains
- [ ] Compiles clean

#### 6.2 Clean Up Unused Types
**Files:**
- Remove `FieldKind::Gradient`, `Offset`, `AnimKind`, `Inverted` if unused
- Remove old proxy type aliases
- Remove dead helper functions

**Acceptance Criteria:**
- [ ] No dead code warnings
- [ ] `cargo clippy` clean
- [ ] Tests pass

#### 6.3 Update LUA_GUIDE.md
**File:** `niri-lua/LUA_GUIDE.md`

Document new config API:
- PropertyRegistry architecture
- ConfigProxy usage
- Signal emission (`config::<path>` events)
- `:snapshot()` method
- Enum string handling

**Acceptance Criteria:**
- [ ] Config section updated
- [ ] Examples accurate
- [ ] Signal emission documented

#### 6.4 Update LUA_SPECIFICATION.md
**File:** `niri-lua/LUA_SPECIFICATION.md`

Update specification with:
- ConfigProxy metamethod behavior
- Property types
- Error formats
- Signal emission

**Acceptance Criteria:**
- [ ] Spec matches implementation
- [ ] Edge cases documented
- [ ] Examples work

#### 6.5 Create Migration Notes
**File:** `docs/projects/statehandle-refactor/migration-notes.md`

Document:
- API changes (if any)
- Breaking changes (if any)
- Performance improvements
- New features (signals)

**Acceptance Criteria:**
- [ ] Changes documented
- [ ] Migration path clear
- [ ] Performance data included

#### 6.6 Final Integration Tests
**File:** `niri-lua/tests/config_api_integration_test.rs`

Comprehensive tests:
- All config paths accessible
- All types work (primitives, enums, arrays, nested)
- Signal emission
- Error handling
- `:snapshot()` and `__pairs`

**Acceptance Criteria:**
- [ ] >90% path coverage
- [ ] All edge cases tested
- [ ] Performance benchmark passing

### Phase 6 Deliverables
- Dead code removed
- Documentation updated
- Migration notes complete
- All tests passing
- Ready for release

---

## File Change Summary

### New Files
| File | Lines (est.) | Description |
|------|--------------|-------------|
| `niri-lua/src/property_registry.rs` | ~250 | PropertyRegistry, PropertyDescriptor, helpers |
| `niri-lua/src/config_proxy.rs` | ~200 | ConfigProxy UserData |
| `niri-lua-derive/src/config_properties.rs` | ~500 | ConfigProperties derive macro |
| `niri-lua/tests/property_registry_tests.rs` | ~150 | Unit tests |
| `niri-lua/tests/config_api_integration_test.rs` | ~200 | Integration tests |
| `docs/projects/statehandle-refactor/migration-notes.md` | ~100 | Migration documentation |

### Modified Files
| File | Change | Impact |
|------|--------|--------|
| `niri-lua/src/lib.rs` | Add registry init | Low |
| `niri-lua/src/config_state.rs` | Arc→Rc | Medium |
| `niri-lua/src/config_wrapper.rs` | Simplify, Arc→Rc | High |
| `niri-lua/src/events_proxy.rs` | Arc→Rc | Medium |
| `niri-lua/src/state_handle.rs` | Arc→Rc | Medium |
| `niri-lua-derive/src/lib.rs` | Add ConfigProperties | Low |
| `niri-lua-traits/src/lib.rs` | Add trait | Low |
| `niri-config/src/*.rs` | Add derives (40 files) | Medium |
| `niri-lua/LUA_GUIDE.md` | Update docs | Low |
| `niri-lua/LUA_SPECIFICATION.md` | Update spec | Low |

### Removed Files
| File | Lines Removed | Reason |
|------|---------------|--------|
| `niri-lua/src/config_proxies.rs` | ~1500 | 40 proxy structs replaced |
| `niri-lua-derive/src/config_proxy.rs` | ~800 | Old macro replaced |

### Net Code Change
- **Added:** ~1400 lines
- **Removed:** ~2300 lines
- **Net:** -900 lines

---

## Risk Assessment

### High Risk
1. **Breaking Lua API** - Mitigated by extensive testing and parallel existence period
2. **Rc<RefCell> panics** - Mitigated by careful borrow analysis and tests

### Medium Risk
1. **Performance regression** - Mitigated by benchmarks in Phase 4
2. **Missing config paths** - Mitigated by completeness test in Phase 3
3. **Signal emission overhead** - Mitigated by optional `no_signal` attribute

### Low Risk
1. **Documentation drift** - Mitigated by final doc update in Phase 6
2. **Example script breakage** - Mitigated by running all examples in Phase 4

---

## Success Metrics

| Metric | Target | Measurement |
|--------|--------|-------------|
| Lines of code | -900 net | `wc -l` before/after |
| Config access latency | <10% regression | Benchmark test |
| Test coverage | >90% config paths | Integration test |
| Compilation time | No significant increase | `cargo build --timings` |
| Memory usage | No increase | Runtime measurement |

---

## Dependencies

- **Phase 2 depends on Phase 1** (macro needs types)
- **Phase 3 depends on Phase 2** (annotation needs macro)
- **Phase 4 depends on Phase 3** (switch needs complete registry)
- **Phase 5 can run parallel to Phase 4** (independent refactoring)
- **Phase 6 depends on Phases 4 & 5** (cleanup after both complete)

---

## Timeline

| Week | Phase | Effort |
|------|-------|--------|
| 1 | Phase 1: Infrastructure | 16-20 hours |
| 2 | Phase 2: Derive Macro | 20-24 hours |
| 3 | Phase 3: Registration | 12-16 hours |
| 4 | Phase 4: Switch | 16-20 hours |
| 5 | Phase 5: Arc→Rc | 12-16 hours |
| 6 | Phase 6: Cleanup | 8-12 hours |

**Total Estimated Effort:** 84-108 hours (10-14 person-days)

---

## Appendix A: Complete Config Path List

See generated list from `PropertyRegistry::init()` debug output after Phase 3.

## Appendix B: DirtyFlag Mapping

| Prefix | DirtyFlag |
|--------|-----------|
| cursor | Cursor |
| layout | Layout |
| animations | Animations |
| input | Input |
| gestures | Gestures |
| overview | Overview |
| recent_windows | RecentWindows |
| clipboard | Clipboard |
| hotkey_overlay | HotkeyOverlay |
| config_notification | ConfigNotification |
| debug | Debug |
| xwayland_satellite | XwaylandSatellite |
| window_rules | WindowRules |
| layer_rules | LayerRules |
| binds | Binds |
| switch_events | SwitchEvents |
| workspaces | Workspaces |
| environment | Environment |
| spawn_at_startup | SpawnAtStartup |
| outputs | Outputs |
| * (default) | Misc |

## Appendix C: Signal Event Format

```lua
niri.events:on("config::cursor.xcursor_size", function(payload)
    -- payload.path = "cursor.xcursor_size"
    -- payload.value = 24 (new value)
    print("Config changed:", payload.path, "=", payload.value)
end)
```
