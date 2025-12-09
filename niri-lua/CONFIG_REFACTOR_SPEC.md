# Config Pipeline Refactoring Specification

## Executive Summary

Refactor the Lua config pipeline from a JSON-based intermediary approach to a direct `UserData` newtype wrapper, reducing ~4,300 LOC to ~500-800 LOC while preserving existing behavior.

## Current State

### Architecture (4,300+ LOC)

```
Lua Table → lua_value_to_json() → serde_json::Value → PendingConfigChanges HashMap
    → apply_*_scalar_change() → NiriConfig
```

**Files involved:**
- `config_proxy.rs` (1,620 LOC) - Lua ↔ JSON conversion, proxy UserData types
- `config_converter.rs` (2,520 LOC) - JSON → NiriConfig conversion (12 `apply_*_scalar_change` functions)
- `extractors.rs` (874 LOC) - Direct Lua → typed extraction (underutilized)

### Why the Deferred Update Model Exists

The current deferred model buffers changes in `PendingConfigChanges` before applying. This exists for:

1. **mlua's `Send + Sync` requirement**: The `send` feature is enabled, requiring all `UserData` types to be `Send + Sync`. Since `niri_config::Config` uses `Rc<RefCell<>>` internally (not thread-safe), it cannot be passed directly to Lua.

2. **Borrow conflict avoidance**: The Config might be borrowed elsewhere when Lua code executes.

3. **Batched change detection**: `apply_pending_lua_config()` compares old vs new values to determine which subsystems need updates (layout recalc, cursor reload, keyboard config, etc.).

**Important**: Despite `Arc<Mutex>` usage, everything runs on a **single thread**. The IPC command `niri msg lua -- "niri.config.x = y"` uses `insert_idle` to execute Lua on the main event loop thread. The thread-safety machinery is required by mlua, not by actual concurrent access.

### How the Current IPC Flow Works

When a user runs `niri msg lua -- "niri.config.prefer_no_csd = true"`:

1. **IPC client** sends `ExecuteLua { code: "niri.config.prefer_no_csd = true" }`
2. **IPC server** uses `insert_idle` to schedule Lua execution on the **main event loop thread**
3. **Lua executes** and writes to `PendingConfigChanges` (via JSON intermediary)
4. **`apply_pending_lua_config()`** is called right after to sync changes to the real Config
5. **Response** sent back to IPC client

### Changes to the Deferred Model

The refactoring **eliminates the deferred buffering** while keeping the same external behavior:

**Current flow (deferred + JSON):**
```
Lua write → lua_value_to_json() → PendingConfigChanges HashMap → apply_*_scalar_change() → Config
```

**New flow (immediate + direct):**
```
Lua write → ConfigWrapper.set_field() → Config directly (with dirty flags)
```

**Key changes:**

1. **`Arc<Mutex>` stays** - Required by mlua's `send` feature, but now wraps the actual `Config` instead of JSON values

2. **Apply immediately** - When Lua sets a field, the Config is updated directly (under mutex lock)

3. **Dirty flags replace buffering** - Instead of buffering changes, we set dirty flags for subsystem updates

4. **Compositor polls dirty flags** - Same as current `apply_pending_lua_config()` but simpler

**New IPC flow after refactoring:**

When a user runs `niri msg lua -- "niri.config.prefer_no_csd = true"`:

1. **IPC client** sends `ExecuteLua { code: "niri.config.prefer_no_csd = true" }`
2. **IPC server** uses `insert_idle` to schedule Lua execution on the **main event loop thread**
3. **Lua executes**:
   - Locks `ConfigWrapper.config` mutex
   - Sets `config.prefer_no_csd = true` directly
   - Sets `dirty.misc = true` flag
   - Unlocks mutex
4. **Compositor processes dirty flags** (replaces `apply_pending_lua_config()`)
5. **Response** sent back to IPC client

**Why this is better:**

| Aspect | Before (Deferred) | After (Immediate) |
|--------|-------------------|-------------------|
| Intermediate storage | JSON in HashMap | None |
| Type conversions | Lua → JSON → Config | Lua → Config |
| Change detection | Compare old/new JSON | Dirty flags |
| Code complexity | 4,300+ LOC | ~500-800 LOC |
| Runtime overhead | Multiple allocations | Minimal |

### Current Behavior to Preserve

```bash
# This must continue to work:
niri msg lua -- "niri.config.prefer_no_csd = true"
niri msg lua -- "niri.config.layout.gaps = 16"
niri msg lua -- "niri.config.input.touchpad.natural_scroll = true"
```

The command:
1. Sends `ExecuteLua` request via IPC
2. Compositor executes Lua code on main thread
3. Config changes are applied
4. Subsystems are notified/refreshed as needed
5. Response returned to IPC client

## Target State

### New Architecture (~500-800 LOC)

```
Lua → ConfigWrapper(Arc<Mutex<Config>>) with UserData impl → Direct field access
    → Dirty flags → Subsystem refresh
```

**New files:**
- `config_wrapper.rs` (~400-600 LOC) - Newtype wrapper with `UserData` implementation
- `config_dirty.rs` (~100-200 LOC) - Dirty flag tracking for subsystem updates

**Files to delete:**
- `config_proxy.rs` - Replaced by `config_wrapper.rs`
- `config_converter.rs` - No longer needed (direct access)

**Files to keep (modified):**
- `extractors.rs` - May be useful for complex type conversions

### Newtype Wrapper Design

```rust
// config_wrapper.rs

use std::sync::{Arc, Mutex};
use mlua::prelude::*;
use niri_config::Config;

/// Tracks which config subsystems have been modified
#[derive(Default)]
pub struct ConfigDirtyFlags {
    pub layout: bool,
    pub input: bool,
    pub cursor: bool,
    pub keyboard: bool,
    pub outputs: bool,
    pub animations: bool,
    pub window_rules: bool,
    // ... etc
}

/// Wrapper around Config that implements UserData for Lua access
pub struct ConfigWrapper {
    pub config: Arc<Mutex<Config>>,
    pub dirty: Arc<Mutex<ConfigDirtyFlags>>,
}

impl ConfigWrapper {
    pub fn new(config: Arc<Mutex<Config>>) -> Self {
        Self {
            config,
            dirty: Arc::new(Mutex::new(ConfigDirtyFlags::default())),
        }
    }
    
    /// Take and reset dirty flags (called by compositor after processing)
    pub fn take_dirty_flags(&self) -> ConfigDirtyFlags {
        std::mem::take(&mut *self.dirty.lock().unwrap())
    }
}

impl UserData for ConfigWrapper {
    fn add_fields<F: UserDataFields<Self>>(fields: &mut F) {
        // Layout section
        fields.add_field_method_get("layout", |lua, this| {
            // Return a LayoutProxy that references the same config
            Ok(LayoutProxy { 
                config: this.config.clone(),
                dirty: this.dirty.clone(),
            })
        });
        
        // Input section
        fields.add_field_method_get("input", |lua, this| {
            Ok(InputProxy { 
                config: this.config.clone(),
                dirty: this.dirty.clone(),
            })
        });
        
        // ... other sections
    }
}

/// Proxy for layout config section
struct LayoutProxy {
    config: Arc<Mutex<Config>>,
    dirty: Arc<Mutex<ConfigDirtyFlags>>,
}

impl UserData for LayoutProxy {
    fn add_fields<F: UserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("gaps", |_, this| {
            Ok(this.config.lock().unwrap().layout.gaps)
        });
        
        fields.add_field_method_set("gaps", |_, this, value: f64| {
            this.config.lock().unwrap().layout.gaps = value;
            this.dirty.lock().unwrap().layout = true;
            Ok(())
        });
        
        // ... other layout fields
    }
}
```

### Compositor Integration

```rust
// In src/niri.rs or similar

impl Niri {
    /// Called after Lua execution to process any config changes
    pub fn process_config_dirty_flags(&mut self) {
        let flags = self.lua_runtime.config_wrapper().take_dirty_flags();
        
        if flags.layout {
            self.layout.update_config(&self.config.borrow());
        }
        if flags.cursor {
            self.reload_cursor();
        }
        if flags.keyboard {
            self.reload_keyboard_config();
        }
        // ... etc
    }
}
```

## Migration Plan

### Phase 1: Add Tests for Current Behavior (In Progress)

Before any refactoring, ensure comprehensive test coverage of current behavior.

**Tests to add:**
- [x] JSON helper function tests (21 tests added)
- [ ] Direct unit tests for `apply_*_scalar_change` functions (started, ~30 tests)
- [ ] Integration tests for IPC `ExecuteLua` config changes
- [ ] Edge case tests (invalid values, missing fields, type coercion)

**Goal**: Establish behavioral baseline so refactoring can be validated against it.

### Phase 2: Create Newtype Wrapper Infrastructure

1. Create `config_wrapper.rs` with `ConfigWrapper` struct
2. Create `ConfigDirtyFlags` for tracking changes
3. Implement `UserData` for `ConfigWrapper` with section proxies
4. Keep old `config_proxy.rs` alongside (feature-flagged)

### Phase 3: Implement Section Proxies

Implement one section at a time, validating against existing tests:

1. **LayoutProxy** - gaps, struts, focus_ring, border, center_focused_column, etc.
2. **InputProxy** - keyboard, touchpad, mouse, tablet, touch settings
3. **CursorProxy** - xcursor_theme, xcursor_size, hide_when_typing
4. **OutputProxy** - per-output settings (more complex, may need collection handling)
5. **AnimationsProxy** - animation curves and durations
6. **BindingsProxy** - key bindings (complex, may need special handling)
7. **WindowRulesProxy** - window rules (complex, collection-based)

### Phase 4: Integrate with Compositor

1. Replace `PendingConfigChanges` polling with dirty flag polling
2. Update `apply_pending_lua_config()` call sites to use `process_config_dirty_flags()`
3. Update IPC `ExecuteLua` handler

### Phase 5: Remove Old Infrastructure

1. Delete `config_proxy.rs`
2. Delete `config_converter.rs`
3. Remove `serde_json` dependency if no longer needed
4. Update documentation and AGENTS.md

## Behavioral Guarantees

The refactoring MUST preserve:

1. **API Compatibility**: `niri.config.layout.gaps = 16` syntax unchanged
2. **IPC Functionality**: `niri msg lua -- "..."` works identically
3. **Type Coercion**: Numbers work as both int and float where appropriate
4. **Error Handling**: Invalid values return Lua errors, don't crash compositor
5. **Subsystem Updates**: Changing layout triggers layout recalc, etc.

## Estimated Impact

| Metric | Before | After | Change |
|--------|--------|-------|--------|
| Total LOC | ~4,300 | ~500-800 | -80% |
| JSON conversions | 92+ calls | 0 | -100% |
| Intermediate allocations | Many (JSON Values) | None | Significant |
| Type safety | Runtime (JSON parsing) | Compile-time | Improved |
| Code complexity | High (multiple layers) | Low (direct access) | Improved |

## Risks and Mitigations

| Risk | Mitigation |
|------|------------|
| Breaking existing behavior | Comprehensive test suite before refactoring |
| Complex nested types | Implement section-by-section with validation |
| Collection types (outputs, bindings) | May need special proxy types for iteration |
| Error messages change | Document expected error formats in tests |

## Success Criteria

1. All existing tests pass
2. `niri msg lua` config changes work identically
3. No performance regression
4. ~80% LOC reduction achieved
5. No new dependencies added

## Open Questions

1. **Collection handling**: How to handle `niri.config.outputs` which is a collection? Options:
   - Return iterator/table of OutputProxy objects
   - Use `__index` metamethod for named access

2. **Bindings**: Key bindings are complex. Options:
   - Keep specialized binding handling
   - Create BindingProxy with special methods

3. **Window rules**: Similar to bindings, may need special handling.

4. **Read-only fields**: Some config values may be computed or read-only. Need to document.
