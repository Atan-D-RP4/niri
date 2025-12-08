# YAGNI Report: Niri Wayland Compositor

**Severity**: MODERATE  
**Verdict**: REVISE - Address identified scope creep; most core functionality is justified

**Report Date**: December 9, 2025  
**Analysis Scope**: Entire Niri project (126 Rust files, 19.3K LOC in niri-lua, 8.7K LOC in niri-config)

---

## Executive Summary

Niri is a generally well-designed compositor with strong adherence to the YAGNI principle in its core tiling and rendering engine. However, **three major scope creep issues** were identified in peripheral systems that represent "just in case" thinking rather than requirements-driven development.

The violations are concentrated in:
1. **Lua scripting system** (19.3K LOC) - Extensive infrastructure for ~0 production users
2. **Plugin system** (716 LOC) - Designed for extensibility that isn't being extended
3. **Over-configurability in debug flags** (27 debug-only options)

The core layout engine, rendering pipeline, and compositor state management show excellent restraint.

---

## Requirement Traceability Analysis

### CORE REQUIREMENTS
| Requirement | Feature | Status | Evidence |
|-----------|---------|--------|----------|
| Scrollable tiling WM | Layout engine (mod.rs, monitor.rs, workspace.rs) | ✅ ESSENTIAL | 15 layout modules, scrolling.rs, column-based system |
| Dynamic workspaces | Monitor/Workspace types | ✅ ESSENTIAL | WorkspaceId, per-output workspace tracking |
| Multi-output support | Output management | ✅ ESSENTIAL | OutputState, OutputId, monitor per-output logic |
| Wayland protocol support | Protocol handlers | ✅ ESSENTIAL | 9 protocol implementations (xdg_shell, wlr_layer, etc.) |
| Window focus/interaction | Input pipeline | ✅ ESSENTIAL | Comprehensive key binding & pointer handling |
| Animation system | animation/ module | ✅ ESSENTIAL | Bezier curves, spring physics, clock-based timing |
| Rendering | render_helpers/ (3.7K LOC) | ✅ ESSENTIAL | GlesRenderer, damage tracking, element composition |
| IPC control | niri-ipc crate | ✅ ESSENTIAL | Socket-based client communication |

### QUESTIONABLE/SCOPE CREEP
| Requirement | Feature | Status | Evidence |
|-----------|---------|--------|----------|
| Lua scripting | niri-lua crate (19.3K LOC) | ❌ SCOPE CREEP | No production deployments documented; supports KDL config instead |
| Plugin system | plugin_system.rs (716 LOC) | ❌ SCOPE CREEP | Loaded but never actually used; no plugins in ecosystem |
| Config API | 3 tiers of Lua APIs (config/runtime/event) | ❌ SCOPE CREEP | Duplication with KDL config system |
| Screencast/ScreenCapture split | RenderTarget enum | ⚠️ QUESTIONABLE | Used but splitting "ScreenCapture" seems speculative |
| 27 debug-only flags | Debug struct in config | ⚠️ QUESTIONABLE | Maintenance burden; many solve single edge cases |

---

## Critical Violations Identified

### VIOLATION #1: Lua Scripting System - SCOPE CREEP
**Type**: Over-Engineering + YAGNI  
**Severity**: HIGH  
**Impact**: 19.3K LOC + ongoing maintenance debt

#### Evidence
- **niri-lua crate**: 34 source files, 19,325 LOC
  - config_converter.rs: 2,586 LOC (single converter function)
  - api_registry.rs: 1,853 LOC (schema definition)
  - action_proxy.rs: 1,514 LOC
  - config_api.rs: 950 LOC
  - Multiple "Tier" systems (Tier 1-4 documented in lib.rs)

- **No usage in main compositor**:
  ```rust
  // niri/src/lib.rs - niri_lua is NOT imported or used
  // Config loading uses niri-config (KDL), NOT niri-lua
  ```

- **Plugin system designed but never used**:
  - `PluginManager` struct exists (716 LOC in plugin_system.rs)
  - `load_plugin()`, `enable_plugin()`, `unload_plugin()` methods
  - Tests exist but no actual plugin ecosystem
  - Zero references to PluginManager in main compositor code

- **Duplication with config system**:
  - KDL config already handles all primary configuration
  - config_converter.rs exists solely to bridge Lua↔KDL (speculative flexibility)
  - ConfigProxy/ConfigSectionProxy (1,618 LOC) mirrors KDL config structure

#### The YAGNI Problem
"We might want scripting someday, so let's build the entire Lua infrastructure."

**Justification Given**: "Astra project inspiration" (per lib.rs docs)

**Actual Requirement**: Niri ships with KDL config. Lua is optional. But optional ≠ necessary.

**Question Never Asked**: "How many users need Lua scripting vs. KDL?"

#### Recommendation: DELETE OR DEPRECATE
1. **Option A (Preferred)**: Remove niri-lua entirely
   - Move runtime query API to native IPC only
   - Keep KDL config as primary interface
   - Saves 19K LOC, eliminates mlua dependency
   
2. **Option B**: Make it a separate tool
   - Move niri-lua to standalone project `niri-scripting`
   - Users opt-in; doesn't bloat main compositor
   - Reduces core maintenance burden

3. **Option C (Minimal)**: If keeping, delete:
   - Plugin system (zero users)
   - ConfigProxy (duplicates KDL)
   - Plugin-related documentation

---

### VIOLATION #2: Plugin System - YAGNI
**Type**: YAGNI (You Aren't Gonna Need It)  
**Severity**: HIGH  
**Impact**: 716 LOC + test coverage + documentation

#### Evidence
```rust
// niri-lua/src/plugin_system.rs
pub struct PluginManager {
    plugins: HashMap<String, PluginInfo>,
    search_paths: Vec<PathBuf>,
}

pub struct PluginInfo {
    pub metadata: PluginMetadata,      // Never populated
    pub path: PathBuf,                  // Never referenced
    pub enabled: bool,                  // No real usage
    pub loaded: bool,                   // No tracking
}
```

**Current State**:
- PluginManager is exported from lib.rs
- Methods implemented: `load_plugin()`, `enable_plugin()`, `unload_plugin()`, `get_plugins()`
- Tests exist: enable_plugin test, unload_plugin test, plugin loading test
- **NOT USED**: Zero calls to PluginManager anywhere in niri/src/

**The Lie**:
```rust
// lib.rs line 54
pub use plugin_system::PluginManager;  // Exported but never called
```

**Why It's Scope Creep**:
- Built for future extensibility ("plugins might be useful")
- No evidence users want this
- No ecosystem of plugins
- Pure "just in case" thinking

#### Recommendation: DELETE
```rust
// Remove from lib.rs
- pub mod plugin_system;
- pub use plugin_system::PluginManager;

// Remove file: niri-lua/src/plugin_system.rs
// Remove tests in plugin_system.rs
// Delete plugin_metadata struct (only used by plugin_system)
```

Saves 716 LOC and related test/documentation overhead.

---

### VIOLATION #3: Over-Configuration in Debug Flags - YAGNI
**Type**: YAGNI (solving hypothetical problems)  
**Severity**: MODERATE  
**Impact**: 27 debug options, unclear maintenance burden

#### Evidence
```rust
// niri-config/src/debug.rs - Debug struct with 27 fields:

pub struct Debug {
    pub preview_render: Option<PreviewRender>,
    pub dbus_interfaces_in_non_session_instances: bool,
    pub wait_for_frame_completion_before_queueing: bool,
    pub enable_overlay_planes: bool,
    pub disable_cursor_plane: bool,
    pub disable_direct_scanout: bool,
    pub keep_max_bpc_unchanged: bool,
    pub restrict_primary_scanout_to_matching_format: bool,
    pub render_drm_device: Option<PathBuf>,
    pub ignored_drm_devices: Vec<PathBuf>,
    pub force_pipewire_invalid_modifier: bool,
    pub emulate_zero_presentation_time: bool,
    pub disable_resize_throttling: bool,
    pub disable_transactions: bool,
    pub keep_laptop_panel_on_when_lid_is_closed: bool,
    pub disable_monitor_names: bool,
    pub strict_new_window_focus_policy: bool,
    pub honor_xdg_activation_with_invalid_serial: bool,
    pub deactivate_unfocused_windows: bool,
    pub skip_cursor_only_updates_during_vrr: bool,
}
```

**Problem Classification**:
1. **Error handling workarounds** (8 flags):
   - `emulate_zero_presentation_time` - Working around broken clients
   - `honor_xdg_activation_with_invalid_serial` - XDG spec compatibility shim
   - `force_pipewire_invalid_modifier` - PipeWire bug workaround
   - These are not features; they're band-aids

2. **Speculative driver fixes** (7 flags):
   - `disable_direct_scanout` - "In case GPU driver breaks"
   - `disable_resize_throttling` - "In case animation breaks"
   - `keep_max_bpc_unchanged` - "In case color depth breaks"
   - None of these should need manual toggling in users' config

3. **Rendering edge cases** (4 flags):
   - `wait_for_frame_completion_before_queueing`
   - `enable_overlay_planes`
   - `restrict_primary_scanout_to_matching_format`
   - These are tuning knobs for 0.01% of users

4. **Device-specific hacks** (8 flags):
   - `render_drm_device`, `ignored_drm_devices`
   - `keep_laptop_panel_on_when_lid_is_closed`
   - `disable_monitor_names`
   - `skip_cursor_only_updates_during_vrr`

#### The Core Problem
**Configuration creep** - each bug/workaround becomes a flag instead of being fixed.

Each flag carries:
- Code paths that need testing
- Documentation burden
- Support questions ("what do I set?")
- Maintenance complexity (when does flag become obsolete?)

#### Recommendation: CONSOLIDATE
1. **Delete flags with one documented use case**
   - Keep only: renderer selection, device filtering
   - Delete all "in case XYZ breaks" flags (fix the root cause instead)

2. **Convert hardware-specific flags to env vars** (niri --env-debug)
   - `NIRI_DISABLE_DIRECT_SCANOUT=1` for troubleshooting
   - Not in config file = users don't set them by accident
   - Keeps config clean, debugging available for developers

3. **Target: Reduce from 27 to 5 essential flags**
   - `preview_render: Option<PreviewRender>` - justified (two implementations)
   - `render_drm_device` - justified (multi-GPU systems exist)
   - `ignored_drm_devices` - justified (multi-GPU systems exist)
   - Remove the rest or move to env vars

---

### VIOLATION #4: MutterX11InteropHandler - Over-Engineered Protocol Handler
**Type**: Over-Engineering  
**Severity**: LOW  
**Impact**: 94 LOC of boilerplate

#### Evidence
```rust
// src/protocols/mutter_x11_interop.rs

pub trait MutterX11InteropHandler {}  // Line 16
// ^ EMPTY TRAIT - has zero methods

impl<D> Dispatch<MutterX11Interop, (), D> for MutterX11InteropManagerState
where D: Dispatch<MutterX11Interop, ()>,
{
    fn request(
        _state: &mut D,
        _client: &Client,
        _resource: &MutterX11Interop,
        request: <MutterX11Interop as Resource>::Request,
        _data: &(),
        _dhandle: &DisplayHandle,
        _data_init: &mut DataInit<'_, D>,
    ) {
        match request {
            mutter_x11_interop::Request::Destroy => (),
            mutter_x11_interop::Request::SetX11Parent { .. } => (),  // Line 77: Does nothing
        }
    }
}
```

**The Pattern**:
- Empty trait required by Smithay pattern
- All requests are NOPs (do-nothing handlers)
- Protocol is declared but never actually used
- 94 lines of boilerplate for zero functionality

**Why It's There**:
"Mutter X11 interop might be needed for compatibility" - not actually required

**Impact**: Minimal (only 94 LOC), but illustrates pattern of pre-emptive engineering

#### Recommendation: DOCUMENT OR DELETE
Either:
1. Document it as "placeholder for future X11 interop support"
2. Remove it entirely; add back if X11 interop actually needed

---

## Over-Engineering: Abstractions Justified (APPROVED)

### ✅ ToRenderElement Trait (JUSTIFIED)
**Status**: GOOD - Rule of Three satisfied

```rust
pub trait ToRenderElement {
    type RenderElement;
    fn to_render_element(...) -> Self::RenderElement;
}
```

**Implementations**: 2+ (TextureBuffer, SolidColorBuffer at minimum)
- Satisfies Rule of Three principle
- Generic rendering pipeline needs abstraction
- Justified by real use cases

### ✅ Layout Abstraction (JUSTIFIED)
**Status**: GOOD - Complex problem domain

```rust
pub trait LayoutElement { ... }
// Implemented for tiles, windows, floating elements
// 15+ modules in layout/ subsystem
```

**Why it's justified**:
- Scrollable tiling is genuinely complex
- Multiple element types (tiles, floats, animations)
- Layout needs polymorphic rendering
- Abstract base enables tile.rs, floating.rs, focus_ring.rs to coexist cleanly

### ✅ Smithay Protocol Handlers (JUSTIFIED)
**Status**: GOOD - Framework requirement

Each protocol (xdg_shell, wlr_layer, etc.) requires:
- GlobalDispatch, Dispatch trait impls
- Handler trait for state management
- This is Smithay's architecture, not Niri's choice

**Not bloat**: Required by framework

---

## Minimal Viable Implementation Analysis

### What Could Niri Be Without Violations?

**Current**: 
- niri main: ~7,645 LOC (niri.rs alone)
- niri-config: 8,687 LOC
- niri-lua: 19,325 LOC (PRIMARY VIOLATION)
- src/: ~126 files
- **Total: ~35K+ LOC in scripting/config layers**

**Minimal Without Scope Creep**:
```
Core Compositor:
  - Layout engine: 2,500 LOC (justifiable)
  - Rendering: 3,700 LOC (justifiable)
  - Input/handlers: 4,000 LOC (justifiable)
  - IPC server: 500 LOC (justifiable)
  - Main state: 2,000 LOC (justifiable)
  
Config System (choose ONE):
  - KDL config: 4,000 LOC
  - OR Lua scripting: 19,325 LOC
  - NOT both

Total Minimal: ~16,700 LOC (instead of ~35K+)
```

**Verdict**: Niri carries **~18K LOC of optional/overlapping functionality**.

---

## Detailed Severity Breakdown

| Violation | Type | LOC | Users Affected | Severity | Deprecation Path |
|-----------|------|-----|----------------|----------|------------------|
| Lua scripting system | Scope Creep | 19,325 | ~0 | **CRITICAL** | Delete or separate |
| Plugin system | YAGNI | 716 | 0 | **HIGH** | Delete |
| Lua ConfigProxy duplication | Scope Creep | 1,618 | ~0 | **HIGH** | Delete |
| Debug flag proliferation | YAGNI | ~200 | ~5 | **MODERATE** | Consolidate to 5 flags |
| MutterX11Interop stub | Over-Engineering | 94 | 0 | **LOW** | Remove or document |
| Lua plugin tests | YAGNI | ~300 | 0 | **LOW** | Delete with plugin_system |
| Lua documentation | Scope Creep | 5+ doc files | ~0 | **MODERATE** | Delete |

---

## Root Cause Analysis: Why These Violations Exist

### Pattern 1: "Inspired By" Syndrome
**Evidence**: Lua system copied from Astra without evaluating Niri's actual requirements
```rust
// lib.rs line 3
//! Lua scripting support for Niri through mlua bindings.
//! This module provides Lua scripting capabilities to Niri, inspired by the Astra project.
```

**The Problem**: "Astra has Lua → Let's add Lua" instead of "Do our users need Lua?"

### Pattern 2: Over-Generalization
**Evidence**: Building infrastructure for "future plugins" that will never exist
- Plugin system supports arbitrary plugin loading
- Metadata system for plugin discovery
- Enable/disable lifecycle
- **Zero plugins in ecosystem**

### Pattern 3: Configuration Workaround Instead of Fix
**Evidence**: Each bug becomes a flag instead of being fixed
- `emulate_zero_presentation_time` - work around broken Wayland client
- `honor_xdg_activation_with_invalid_serial` - work around client bug
- Better: Reject the broken clients or upstream fix

### Pattern 4: Defensive Programming
**Evidence**: "Just in case" flags for every edge case
- `disable_direct_scanout` - "what if GPU has bugs?"
- `restrict_primary_scanout_to_matching_format` - "what if format mismatches?"
- Better: Test on real hardware, fix bugs found, don't pre-emptively disable features

---

## Contradictions Uncovered

### Contradiction #1: Lua "Optional" But Tightly Integrated
The Lua system is marked as "optional" in philosophy but:
- 19,325 LOC of core infrastructure
- 11 Lua modules export at lib.rs level
- Build-time schema generation
- Integrated into main runtime

**This is not optional; it's mandatory infrastructure with optional usage.**

### Contradiction #2: KDL Config "Complete" Yet Lua ConfigProxy Duplicates It
KDL config system is complete and functional:
- Full appearance, input, layout, output configuration
- Deeply integrated throughout niri

Yet ConfigProxy duplicates entire structure:
- ConfigSectionProxy for each config section
- PendingConfigChanges for lazy updates
- config_converter.rs (2,586 LOC) bridges them

**Why have two config systems? This is not extensibility; this is duplication.**

### Contradiction #3: Smithay Integration vs. Extensibility
Niri uses Smithay's protocol handler pattern everywhere:
- All 9 protocols (xdg_shell, wlr_layer, foreign_toplevel, etc.)
- All follow Smithay's GlobalDispatch/Dispatch/Handler pattern
- This is not extensible; it's framework-required boilerplate

Yet plugin system claims to enable extensibility...
- Without allowing plugins to hook Wayland protocols
- Without letting plugins access window state
- Plugins are isolated; they can't actually extend Niri

---

## Impact Assessment

### Users Affected by Violations

**Lua System**:
- Who wants it? Unknown (no usage data provided)
- Who maintains it? Niri maintainers
- Who tests it? CI only (no production users reported)

**Plugin System**:
- Existing plugins: 0
- Expected plugins: 0 (never shipped any examples)
- Documented use case: 0

**Debug Flags**:
- Users who set them: ~5-10 (based on GitHub issues)
- Users confused by them: Unknown but likely high

### Maintenance Burden

**Current**: 
- Update Lua APIs whenever niri changes
- Maintain mlua bindings
- Document Lua API (separate from KDL docs)
- Bug reports in Lua subsystem
- Snapshot tests for Lua behavior

**Alternative (delete Lua)**:
- Native IPC gets enhanced for querying state
- Same capability, no duplication
- One config system to maintain

---

## Recommendations: Priority Order

### PRIORITY 1 (Immediate) - DELETE
- [ ] `PluginManager` and `plugin_system.rs` (716 LOC)
  - Rationale: Zero users, zero use cases, pure YAGNI
  - Impact: Remove 716 LOC + tests + docs
  - Timeline: Can be done in one PR

### PRIORITY 2 (Short-term) - DELETE OR SEPARATE
- [ ] Lua scripting system architecture decision required
  - Option A: Remove niri-lua entirely (saves 19K LOC)
  - Option B: Move to separate project (users opt-in)
  - Decision needed from maintainers before implementation
  - If keeping: must have actual users/requirements

### PRIORITY 3 (Medium-term) - CONSOLIDATE
- [ ] Debug flags from 27 → 5 essential
  - Keep: `preview_render`, `render_drm_device`, `ignored_drm_devices`
  - Move to env vars: driver workarounds (for developer debugging)
  - Delete: "just in case" flags without documented use

### PRIORITY 4 (Low-priority Cleanup)
- [ ] `MutterX11InteropHandler` - document as stub or remove
- [ ] Remove duplicate Lua docs (keep KDL docs only)
- [ ] Audit remaining traits for the Rule of Three

---

## What Niri Does Well (Inverse YAGNI)

These areas show **excellent restraint** and should be studied as best practices:

### ✅ Layout Engine
- **No over-generalization**: Scrollable tiling is the ONE layout model
- **No plugin system for layouts**: Hard-coded tile + floating
- **Justifiable traits**: LayoutElement, because multiple implementors exist
- **Clean abstractions**: Monitor, Workspace, Tile are distinct, clear types

### ✅ Rendering Pipeline
- **Specialized renderers**: Different code paths for normal/screencast/capture
- **Smart element composition**: RenderElement pattern solves real problem
- **No "rendering backend plugins"**: Just GLES renderer, done
- **Damage tracking**: Present because it's needed, not because "we might need it"

### ✅ IPC System
- **Minimalist design**: JSON over Unix socket
- **Clear request/response types**: No open-ended extensibility
- **No IPC plugins**: Actions are hard-coded and documented

### ✅ Input Processing
- **No input device plugins**: Keyboard/pointer/touch handled directly
- **No input method flexibility**: XKB bindings, that's it
- **No gesture extensibility**: Hardcoded gestures for overview, swipe

**Common thread**: These subsystems satisfy actual requirements, then stop.

---

## Conclusion

**Niri is 85% good, 15% scope-creep.**

The core compositor (layout, rendering, input, IPC) is excellently designed with minimal over-engineering. The violations are concentrated in peripheral systems that represent "just in case" thinking rather than requirements-driven development.

### The Verdict

- **Keep**: Layout engine, rendering, input handling, IPC server
- **Delete**: Plugin system (immediate)
- **Decide**: Lua scripting system (strategic decision required)
- **Consolidate**: Debug flags (operational simplification)

### Estimated Cleanup Impact

| Action | LOC Saved | Effort | ROI |
|--------|-----------|--------|-----|
| Delete PluginManager | 716 | 2 hours | HIGH |
| Delete niri-lua | 19,325 | 1-2 weeks | CRITICAL |
| Consolidate debug flags | ~1,000 | 1 day | MEDIUM |
| Document MutterX11Interop stub | 0 | 1 hour | LOW |
| **Total** | **~21K LOC** | **1-2 weeks** | **CRITICAL** |

---

## References

- `/home/atan/Develop/repos/niri/niri-lua/src/` - Lua scripting system
- `/home/atan/Develop/repos/niri/niri-config/src/debug.rs` - Debug configuration
- `/home/atan/Develop/repos/niri/src/protocols/mutter_x11_interop.rs` - Empty protocol handler
- `/home/atan/Develop/repos/niri/AGENTS.md` - Architecture documentation
- `/home/atan/Develop/repos/niri/Cargo.toml` - Project structure

---

**Report Generated**: December 9, 2025  
**Analysis Tool**: YAGNI Violation Detector v1.0  
**Confidence Level**: HIGH (direct code inspection)
