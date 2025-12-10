## YAGNI Report: niri-lua Crate

**Analysis Date**: 2025-02-10  
**Severity**: MODERATE  
**Verdict**: REVISE - Remove unintegrated subsystems, simplify indirection layer

---

## Executive Summary

The niri-lua crate is ~19.8K LOC implementing Lua scripting for Niri. Core features (config API, events, actions, state queries) are **REQUIRED and well-integrated**. However, two substantial subsystems are **UNINTEGRATED YAGNI VIOLATIONS**:

1. **Plugin System** (716 LOC): Fully implemented, unused, no integration path
2. **Module Loader** (139+ LOC): Fully implemented, unused, no integration path

Additionally, the **Configuration Conversion Layer** (2,586 LOC) uses an inefficient `serde_json::Value` intermediary that adds indirection without clear benefit.

---

## Requirement Traceability

### MUST-HAVE Features (from README & AGENTS.md)

| Feature | Status | LOC | Integrated | Evidence |
|---------|--------|-----|-----------|----------|
| Configuration API (read-only KDL exposure) | ✅ Complete | 950 | YES | `config_api.rs` - wired in `src/main.rs` |
| Reactive config proxy (`niri.config.*`) | ✅ Complete | 1,620 | YES | `config_proxy.rs` - wired in main |
| Config conversion (Lua→Niri) | ✅ Complete | 2,586 | YES | `config_converter.rs` - called on startup |
| Action execution (`niri.action:*()`) | ✅ Complete | 1,514 | YES | `action_proxy.rs` - full 90 actions |
| Event system (`niri.events:on/once/off`) | ✅ Complete | 496 | YES | `events_proxy.rs` - 23+ events wired |
| Event emission (`event_system.rs`) | ✅ Complete | 91 | YES | `event_system.rs` - wired in compositor |
| State queries (`niri.state.*`) | ✅ Complete | 909 | YES | `runtime_api.rs` - handles deadlock-safe queries |
| Timers/loop API (`niri.loop.new_timer()`) | ✅ Complete | 793 | YES | `loop_api.rs` - integrated with calloop |
| REPL via IPC | ✅ Complete | 538 | YES | `ipc_bridge.rs`, `ipc_repl.rs` |
| LSP type generation | ✅ Complete | 2,516 | YES | `api_registry.rs` - generates `types/api.lua` |
| **Plugin System** | 🚫 STUB | 716 | **NO** | `plugin_system.rs` - only tests, no wiring |
| **Module Loader** | 🚫 STUB | 139+ | **NO** | `module_loader.rs` - only tests, no wiring |

---

## Critical Violations

### 1. UNINTEGRATED PLUGIN SYSTEM (SCOPE CREEP)

**Severity**: HIGH  
**Classification**: YAGNI + Over-Engineering  
**Evidence**: 
- 716 LOC across plugin discovery, metadata parsing, loading, enable/disable, unload
- 9+ test cases all isolated to `plugin_system.rs` tests
- **Zero integration points** in main compositor code
- README explicitly states: "Plugin System: 🚧 Stub - Discovery works, sandbox/lifecycle not implemented"
- No call sites in `src/` main crate
- No documentation on how plugins would be invoked

**The Problem**:
```rust
// This exists and works in tests...
pub struct PluginManager {
    plugins: HashMap<String, PluginInfo>,
    search_paths: Vec<PathBuf>,
}

impl PluginManager {
    pub fn new() -> Self { ... }
    pub fn add_search_path(&mut self, path: PathBuf) { ... }
    pub fn discover(&mut self, lua: &Lua) -> LuaResult<()> { ... }
    pub fn load_plugin(&mut self, name: &str) -> LuaResult<()> { ... }
    pub fn enable_plugin(&mut self, name: &str) -> bool { ... }
    pub fn disable_plugin(&mut self, name: &str) -> bool { ... }
    pub fn unload_plugin(&mut self, name: &str) -> bool { ... }
    pub fn list_plugins(&self) -> Vec<&PluginInfo> { ... }
}

// ...but it's never instantiated or called anywhere in the compositor
```

**Why It's YAGNI**:
- Planned for Tier 5 (future), but implemented as full feature
- "Infrastructure complete, sandbox/lifecycle not implemented" = incomplete by design
- Plugin system requires security decisions that haven't been made (sandbox model, permissions, lifecycle)
- No specification for how plugins would integrate with the compositor's async model
- Creating plugins before the model is finalized locks in decisions prematurely

**Recommendation**: **DELETE entirely**. If plugins become a requirement, rebuild with clear specification in a separate RFC. Current implementation is guessing about a future that hasn't been designed.

---

### 2. UNINTEGRATED MODULE LOADER (SCOPE CREEP)

**Severity**: MODERATE  
**Classification**: YAGNI  
**Evidence**:
- 139 LOC of `module_loader.rs` with full `require()` support
- Searches 5+ directories: `~/.config/niri/plugins`, `~/.local/share/niri/plugins`, `/usr/local/share/niri/plugins`, `/usr/share/niri/plugins`, `.`
- Only called in tests, never wired to runtime
- README doesn't mention module loading as a feature
- Conflicts with/depends on non-existent plugin system

**The Problem**:
```rust
pub struct ModuleLoader {
    search_paths: Vec<PathBuf>,
}

impl ModuleLoader {
    pub fn new() -> Self { ... }
    pub fn with_paths(paths: Vec<PathBuf>) -> Self { ... }
    pub fn add_search_path(&mut self, path: PathBuf) { ... }
    pub fn find_module(&self, module_name: &str) -> Option<PathBuf> { ... }
    pub fn setup_module_paths(&self, lua: &Lua) -> LuaResult<()> { ... }
}

// Never called from main.rs or anywhere in the compositor
```

**Why It's YAGNI**:
- Lua's standard `require()` works fine without custom module loader
- Only useful if plugins exist (which they don't, see violation #1)
- No documentation on expected plugin directory structure
- Hardcoded search paths assume plugin directories that don't exist in packaged Niri

**Recommendation**: **DELETE**. If needed, implement when actual plugins are designed. Standard Lua `require()` is sufficient for user-config modules.

---

### 3. CONFIG CONVERTER INDIRECTION LAYER (Over-Engineering)

**Severity**: MODERATE  
**Classification**: Architectural bloat + poor abstraction  
**Evidence**:
- **2,586 LOC** converting Lua → JSON → Niri Config structures
- Acts as middleware: `LuaValue → serde_json::Value → Config`
- README's own TODO: "**TODO: Simplify config_proxy.rs** - The config proxy uses `serde_json::Value` as an intermediary format. Consider whether direct Lua-to-Config conversion would be more efficient."
- Extractors module (874 LOC) provides `extract_*` helpers that duplicate what could be direct conversion
- Validators module (868 LOC) validates JSON values that could be validated during direct conversion

**The Problem - Unnecessary Indirection**:

Current architecture:
```
Lua config proxy sets via metatables
    ↓
PendingConfigChanges stores as serde_json::Value
    ↓
config_converter.rs reads JSON and converts to Config
    ↓
Validators check JSON values retrospectively
    ↓
Finally applies to actual Config struct
```

Why this is broken:
1. **Double parsing**: Extract from Lua → serialize to JSON → deserialize from JSON → convert to Config types
2. **Late validation**: JSON is validated AFTER collection in PendingConfigChanges, not during set operations
3. **Lossy conversion**: Lua → JSON loses type information (e.g., distinguishing `nil` from missing keys)
4. **Harder to debug**: Three layers of transformation before reaching Config

**Evidence in Code**:
```rust
// config_proxy.rs - stores as JSON
pub struct PendingConfigChanges {
    pub scalar_changes: HashMap<String, serde_json::Value>,
    pub collection_additions: HashMap<String, Vec<serde_json::Value>>,
    pub collection_removals: HashMap<String, Vec<serde_json::Value>>,
    pub collection_replacements: HashMap<String, Vec<serde_json::Value>>,
}

// Then config_converter.rs has to figure out what the JSON actually means:
pub fn apply_pending_lua_config(runtime: &LuaRuntime, config: &mut Config) -> anyhow::Result<()> {
    let pending = runtime.get_pending_config_changes()?;
    
    // Has to iterate through scalar_changes and handle each path separately
    for (path, value) in &pending.scalar_changes {
        // 2,586 LOC of custom parsing logic
    }
}

// validators.rs duplicates conversion logic for validation:
pub struct ConfigValidator;
impl ConfigValidator {
    pub fn validate_scalar(&mut self, path: &str, value: &serde_json::Value) -> Result<(), String> {
        // Validates AFTER extraction, not during
    }
}
```

**Why It's Over-Engineered**:
- JSON intermediary adds ~1,700 LOC (extractors + validators + converter) for no benefit
- Lua values could be validated immediately in `config_proxy.rs` setters
- Config struct could be built directly from LuaValue with proper type checking
- The "flexibility" of JSON indirection is a non-requirement (never used polymorphically)

**Recommendation**: 
- **SIMPLIFY** to direct Lua → Config conversion
- Move validation to `config_proxy.rs` setters (fail fast)
- Remove `serde_json::Value` from `PendingConfigChanges`
- Store `LuaValue` or typed enum variants instead
- Saves ~1,000 LOC and improves error messages

---

## Feature Inventory & Classification

| Feature | Lines | Type | Status | Notes |
|---------|-------|------|--------|-------|
| **REQUIRED** |  |  |  |  |
| Event system core | 329 | Code | ✅ Integrated | event_handlers.rs - essential, well-designed |
| Events proxy API | 496 | Code | ✅ Integrated | events_proxy.rs - on/once/off perfectly scoped |
| Config API exposure | 950 | Code | ✅ Integrated | config_api.rs - read-only, complete KDL coverage |
| Config proxy (reactive) | 1,620 | Code | ✅ Integrated | config_proxy.rs - does too much (see violation #3) |
| Config converter | 2,586 | Code | ✅ Integrated | config_converter.rs - over-engineered (see violation #3) |
| Config extractors | 874 | Code | ✅ Used | extractors.rs - supporting converter, duplication |
| Config validators | 868 | Code | ✅ Used | validators.rs - supporting converter, late validation |
| Action proxy (90 actions) | 1,514 | Code | ✅ Integrated | action_proxy.rs - comprehensive, necessary |
| Runtime API (state queries) | 909 | Code | ✅ Integrated | runtime_api.rs - well-designed deadlock safety |
| Event data structures | 392 | Code | ✅ Used | event_data.rs - necessary for events |
| Event emitter | ? | Code | ⚠️ Ambiguous | event_emitter.rs - README notes "parallel implementations, prune unused" |
| Loop/timer API | 793 | Code | ✅ Integrated | loop_api.rs - essential for non-blocking code |
| IPC bridge | 538 | Code | ✅ Integrated | ipc_bridge.rs + ipc_repl.rs - REPL support |
| API registry/schema | 2,516 | Code | ✅ Integrated | api_registry.rs - generates types/api.lua for LSP |
| Lua types wrappers | 395 | Code | ⚠️ Partial | lua_types.rs - supports validators, some may be dead code |
| **UNINTEGRATED (YAGNI)** |  |  |  |  |
| Plugin system | 716 | Code | ❌ Unintegrated | plugin_system.rs - full impl, zero wiring |
| Module loader | 139+ | Code | ❌ Unintegrated | module_loader.rs - full impl, zero wiring |
| **UNCLEAR/DUPLICATED** |  |  |  |  |
| Event emitter parallel impl? | ? | Design | ⚠️ Needs review | README: "Two parallel implementations, prune unused" |
| Test utils | 478 | Test | ✅ Used | test_utils.rs - only 3 files use it though |

---

## Minimal Viable Implementation

What would a **truly minimal** niri-lua look like if built from scratch focusing only on integrated requirements?

### Tier 1: Event System (Essential)
- `EventHandlers` - dispatch callbacks (KEEP)
- `EventSystem` wrapper - public interface (KEEP)
- `EventsProxy` - Lua `on/once/off` (KEEP)

### Tier 2: Configuration  
- `ConfigApi` - read-only KDL exposure (KEEP)
- **ConfigProxy simplified**: Direct LuaValue → Config conversion, fail-fast validation
  - Move from 3-layer pipeline to 1-layer pipeline
  - Validation happens immediately during set operations
  - ~600 LOC instead of 1,620 + 874 + 2,586 = 5,080
- Remove: `serde_json::Value` indirection, late validators, extractors duplication

### Tier 3: Actions & State
- `ActionProxy` - all 90 actions (KEEP)
- `RuntimeApi` - deadlock-safe state queries (KEEP - well-designed)
- `EventData` - event parameter structures (KEEP)

### Tier 4: Async
- `LoopApi` - timer and defer mechanisms (KEEP)
- `IpcRepl` - REPL via IPC (KEEP)

### Delete Entirely
- `PluginSystem` (716 LOC)
- `ModuleLoader` (139+ LOC)
- `Validators` module (868 LOC) - move to runtime validation
- Most of `Extractors` module (874 LOC) - move to config_proxy setters
- Duplicate event emission logic (if it exists)

### Result: ~11K LOC instead of 19.8K LOC (-44%)

---

## Impact Analysis

### What breaks if we DELETE these?

**Delete PluginManager**: 
- ✅ Nothing breaks - never used
- Users who want plugins must wait for proper design/spec

**Delete ModuleLoader**:
- ✅ Nothing breaks - never integrated
- Users can still use `require()` with standard Lua paths

**Simplify config conversion**:
- ✅ Works better - faster, clearer errors, simpler code
- ❌ May break if someone depended on JSON-layer internals (unlikely - not public API)

---

## Design Questions This Raises

1. **Why implement plugins now without a design spec?**
   - "Just in case" planning violates YAGNI
   - Plugins require: sandbox model, permission system, lifecycle hooks, version management
   - These decisions should be made WITH users, not guessed now

2. **Why does config conversion need 3 layers?**
   - Original comment: "Consider whether direct Lua-to-Config conversion would be more efficient"
   - The answer: YES. Direct conversion is more efficient and clearer

3. **Are there any actual users requesting plugins/modules?**
   - If no: DELETE
   - If yes: Scope a proper design (RFC process) before implementing

4. **What is PendingConfigChanges actually for?**
   - It's a staging layer to collect changes from Lua before applying to Config
   - But why use JSON? Why not store the actual values?
   - Suggests architectural uncertainty about what should be mutable

---

## Severity Breakdown

| Violation | Type | Severity | Action | Impact |
|-----------|------|----------|--------|--------|
| Plugin System unintegrated | Scope Creep | HIGH | DELETE | Free 716 LOC, removes maintenance burden |
| Module Loader unintegrated | Scope Creep | MODERATE | DELETE | Free 139 LOC, no functional loss |
| Config converter indirection | Over-Engineering | MODERATE | REFACTOR | Free 1,700+ LOC, improve perf/clarity |
| Event emitter duplication? | Design Smell | MINOR | REVIEW | Needs investigation per README note |

---

## Recommendations

### Immediate Actions (No Risk)

1. **DELETE `plugin_system.rs` entirely** (716 LOC)
   - Remove from `lib.rs` exports
   - Remove from `Cargo.toml`
   - Update README to remove "Plugin System" from feature table
   - Rationale: Unintegrated, speculative, maintenance overhead

2. **DELETE `module_loader.rs` entirely** (139 LOC)
   - Remove from `lib.rs` exports
   - Update README to remove "Module Loader" mention
   - Rationale: Only useful if plugins exist; standard Lua require is sufficient

3. **Investigate event emitter parallel implementations**
   - README states: "Unify event_emitter.rs - Contains two parallel implementations (Rust struct and Lua-based global tables). Evaluate which is better and prune the unused code."
   - Assess which implementation is active, delete the other

### Medium-term Refactor (High Value)

4. **Simplify configuration conversion pipeline**
   - Remove `serde_json::Value` intermediary from `PendingConfigChanges`
   - Move validation to setter-time in `config_proxy.rs`
   - Combine `extractors.rs` and `validators.rs` logic into `config_proxy.rs`
   - Delete or drastically shrink `config_converter.rs`
   - Target: ~1,000 LOC reduction, improved error reporting

### Future (When Spec Exists)

5. **Plugin System Design Process**
   - If users request: RFC discussion on sandbox model, permissions, lifecycle
   - Design document covering: discovery, loading, unloading, version management, security
   - Only then: implement alongside actual integration points in compositor

---

## Summary Table

```
Classification Summary:
  REQUIRED & Integrated:       ~14.5K LOC (73%) ✅
  REQUIRED but Over-Engineered: ~2.2K LOC (11%) ⚠️ → Should be ~1.2K
  UNINTEGRATED (YAGNI):        ~0.9K LOC (4%)  ❌ DELETE
  UNCLEAR/Duplicated:          ~2.2K LOC (11%) ❌ REVIEW
  
  → Target: 11.5K LOC (-42% reduction)
```

---

## Conclusion

The niri-lua crate successfully implements **core required features** (configuration, events, actions, state, async primitives) that are well-integrated and essential. However, it suffers from:

1. **~900 LOC of unintegrated YAGNI code** (plugins + module loader) taking up maintenance burden with zero usage
2. **~1,700 LOC of over-engineered indirection** in the config conversion layer that could be radically simplified
3. **Potential duplication** in event emission that needs investigation

**Verdict**: REVISE the crate by:
- Removing unintegrated subsystems immediately
- Simplifying the config conversion architecture 
- Keeping all integrated, well-designed components

This would reduce maintenance burden, improve performance, and clarify the remaining API.
