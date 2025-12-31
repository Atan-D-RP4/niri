# StateHandle Refactor Technical Specification
**Brief**: Internal design document based on review session analysis
**Created**: 2024-12-31
**Status**: Draft
**Compliance Score**: N/A (no external brief)
## Executive Summary
Refactor niri-lua's state access architecture from a scoped-context model to an always-live StateHandle abstraction. This eliminates the dual code paths, removes thread-local flags, enables state queries from any Lua context, and adds O(1) targeted queries.
## Data Contracts
### Inputs
| Source | Data | Type | Notes |
|--------|------|------|-------|
| EventStreamState | windows, workspaces, keyboard_layouts | `Rc<RefCell<EventStreamState>>` | Already exists in IPC server |
| IpcOutputMap | outputs | `Arc<Mutex<HashMap<String, Output>>>` | Already exists in backend |
| Pointer events | cursor position | New: `Arc<Mutex<Option<CursorPosition>>>` | Updated on pointer motion |
| Focus changes | focus mode | New: `Arc<Mutex<FocusMode>>` | Updated on focus change |
| Layer shell | reserved space | New: `Arc<Mutex<HashMap<String, ReservedSpace>>>` | Updated on layer change |
### Outputs
| Consumer | Data | Type | Notes |
|----------|------|------|-------|
| Lua scripts | `niri.state.windows()` | `Vec<Window>` as Lua table | Always-live query |
| Lua scripts | `niri.state.window(id)` | `Option<Window>` as Lua table/nil | O(1) targeted query |
| Lua scripts | `niri.state.workspaces()` | `Vec<Workspace>` as Lua table | Always-live query |
| Lua scripts | `niri.state.workspace(ref)` | `Option<Workspace>` as Lua table/nil | O(1) by id, O(n) by name |
| Lua scripts | `niri.state.outputs()` | `Vec<Output>` as Lua table | Always-live query |
| Lua scripts | `niri.state.output(name)` | `Option<Output>` as Lua table/nil | O(1) targeted query |
| Lua scripts | `niri.state.focused_window()` | `Option<Window>` as Lua table/nil | Always-live query |
| Lua scripts | `niri.state.keyboard_layouts()` | `Option<KeyboardLayouts>` as Lua table/nil | Always-live query |
| Lua scripts | `niri.state.cursor_position()` | `Option<CursorPosition>` as Lua table/nil | Always-live query |
| Lua scripts | `niri.state.focus_mode()` | `string` | Always-live query |
| Lua scripts | `niri.state.reserved_space(output)` | `ReservedSpace` as Lua table | Always-live query |
### Interface Constraints
| Constraint | Reason |
|------------|--------|
| `niri.state.*` API surface must not change | Backward compatibility with existing scripts |
| Function signatures must match current implementation | Scripts depend on return types |
| Error messages should be similar | User familiarity |
### Scope Classification
**BROWNFIELD** - Refactoring existing state access system. Must preserve external Lua API.
## Technical Design
### Architecture Overview
```
┌─────────────────────────────────────────────────────────────────────┐
│                         CURRENT ARCHITECTURE                         │
├─────────────────────────────────────────────────────────────────────┤
│                                                                      │
│  ┌──────────────────┐    ┌──────────────────┐                       │
│  │ Event Handler    │    │ Timer/IPC        │                       │
│  │ Context          │    │ Context          │                       │
│  └────────┬─────────┘    └────────┬─────────┘                       │
│           │                       │                                  │
│           ▼                       ▼                                  │
│  ┌──────────────────┐    ┌──────────────────┐                       │
│  │emit_with_scoped_ │    │with_scoped_state │                       │
│  │     state()      │    │       ()         │                       │
│  │  (LIVE queries)  │    │ (SNAPSHOT)       │                       │
│  └────────┬─────────┘    └────────┬─────────┘                       │
│           │                       │                                  │
│           ▼                       ▼                                  │
│  ┌─────────────────────────────────────────┐                        │
│  │         SCOPED_STATE_ACTIVE flag        │                        │
│  │         __niri_scoped_state table       │                        │
│  └─────────────────────────────────────────┘                        │
│                         │                                            │
│                         ▼                                            │
│  ┌─────────────────────────────────────────┐                        │
│  │           niri.state.* functions        │                        │
│  │    (error if called outside scope)      │                        │
│  └─────────────────────────────────────────┘                        │
│                                                                      │
└─────────────────────────────────────────────────────────────────────┘
┌─────────────────────────────────────────────────────────────────────┐
│                          NEW ARCHITECTURE                            │
├─────────────────────────────────────────────────────────────────────┤
│                                                                      │
│  ┌──────────────────────────────────────────────────────────┐       │
│  │                     StateHandle                           │       │
│  │  ┌─────────────────────────────────────────────────────┐ │       │
│  │  │ event_stream_state: Rc<RefCell<EventStreamState>>   │ │       │
│  │  │ output_map: Arc<Mutex<IpcOutputMap>>                │ │       │
│  │  │ cursor_position: Arc<Mutex<Option<CursorPosition>>> │ │       │
│  │  │ focus_mode: Arc<Mutex<FocusMode>>                   │ │       │
│  │  │ reserved_spaces: Arc<Mutex<HashMap<String, Res...>>>│ │       │
│  │  └─────────────────────────────────────────────────────┘ │       │
│  └──────────────────────────────────────────────────────────┘       │
│                         │                                            │
│                         ▼                                            │
│  ┌──────────────────────────────────────────────────────────┐       │
│  │              lua.set_app_data(state_handle)              │       │
│  └──────────────────────────────────────────────────────────┘       │
│                         │                                            │
│           ┌─────────────┼─────────────┬─────────────┐               │
│           ▼             ▼             ▼             ▼               │
│  ┌────────────┐ ┌────────────┐ ┌────────────┐ ┌────────────┐       │
│  │   Event    │ │   Timer    │ │    REPL    │ │    IPC     │       │
│  │  Handler   │ │  Callback  │ │ Execution  │ │ Execution  │       │
│  └────────────┘ └────────────┘ └────────────┘ └────────────┘       │
│           │             │             │             │               │
│           └─────────────┴─────────────┴─────────────┘               │
│                         │                                            │
│                         ▼                                            │
│  ┌──────────────────────────────────────────────────────────┐       │
│  │           niri.state.* functions                         │       │
│  │    (query StateHandle from app_data - ALWAYS WORKS)      │       │
│  └──────────────────────────────────────────────────────────┘       │
│                                                                      │
└─────────────────────────────────────────────────────────────────────┘
```
### Data Models
#### StateHandle (NEW)
```rust
/// A handle that provides always-live access to compositor state.
/// Stored in Lua's app_data, queries resolve through shared references.
#[derive(Clone)]
pub struct StateHandle {
    /// Windows, workspaces, keyboard layouts (already Rc<RefCell<>> in IPC)
    event_stream_state: Rc<RefCell<EventStreamState>>,

    /// Outputs/monitors (already Arc<Mutex<>> in backend)
    output_map: Arc<Mutex<IpcOutputMap>>,

    /// Cursor position - updated on pointer motion
    cursor_position: Arc<Mutex<Option<CursorPosition>>>,

    /// Focus mode - updated on focus change
    focus_mode: Arc<Mutex<FocusMode>>,

    /// Reserved space per output - updated on layer shell changes
    reserved_spaces: Arc<Mutex<HashMap<String, ReservedSpace>>>,
}
```
#### CursorPosition (existing in runtime_api.rs, move to state_handle.rs)
```rust
#[derive(Clone, Debug, Default)]
pub struct CursorPosition {
    pub x: f64,
    pub y: f64,
    pub output: String,
}
```
#### FocusMode (existing in runtime_api.rs, move to state_handle.rs)
```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum FocusMode {
    #[default]
    Normal,
    Overview,
    LayerShell,
    Locked,
}
```
#### ReservedSpace (existing in runtime_api.rs, move to state_handle.rs)
```rust
#[derive(Clone, Debug, Default)]
pub struct ReservedSpace {
    pub top: i32,
    pub bottom: i32,
    pub left: i32,
    pub right: i32,
}
```
### StateHandle Methods
```rust
impl StateHandle {
    /// Create a new StateHandle from existing shared state
    pub fn new(
        event_stream_state: Rc<RefCell<EventStreamState>>,
        output_map: Arc<Mutex<IpcOutputMap>>,
    ) -> Self;
    // === Collection Queries (LIVE) ===

    /// Get all windows
    pub fn windows(&self) -> Vec<Window>;

    /// Get all workspaces
    pub fn workspaces(&self) -> Vec<Workspace>;

    /// Get all outputs
    pub fn outputs(&self) -> Vec<Output>;

    /// Get keyboard layouts
    pub fn keyboard_layouts(&self) -> Option<KeyboardLayouts>;
    // === Targeted Queries (O(1) where possible) ===

    /// Get window by ID - O(1) HashMap lookup
    pub fn window(&self, id: u64) -> Option<Window>;

    /// Get focused window - O(n) but typically small n
    pub fn focused_window(&self) -> Option<Window>;

    /// Get workspace by ID - O(1) HashMap lookup
    pub fn workspace_by_id(&self, id: u64) -> Option<Workspace>;

    /// Get workspace by name - O(n) linear search
    pub fn workspace_by_name(&self, name: &str) -> Option<Workspace>;

    /// Get workspace by index - O(n) linear search
    pub fn workspace_by_idx(&self, idx: u8) -> Option<Workspace>;

    /// Get output by name - O(n) but typically small n
    pub fn output_by_name(&self, name: &str) -> Option<Output>;
    // === Compositor State Queries (LIVE) ===

    /// Get cursor position
    pub fn cursor_position(&self) -> Option<CursorPosition>;

    /// Get focus mode
    pub fn focus_mode(&self) -> FocusMode;

    /// Get reserved space for an output
    pub fn reserved_space(&self, output_name: &str) -> ReservedSpace;
    // === Update Methods (called by compositor) ===

    /// Update cursor position
    pub fn set_cursor_position(&self, pos: Option<CursorPosition>);

    /// Update focus mode
    pub fn set_focus_mode(&self, mode: FocusMode);

    /// Update reserved space for an output
    pub fn set_reserved_space(&self, output_name: &str, space: ReservedSpace);

    /// Remove reserved space entry when output disconnected
    pub fn remove_reserved_space(&self, output_name: &str);
}
```
### Integration Points
#### 1. StateHandle Creation (src/niri.rs)
```rust
// In State::new() after creating IPC server and backend
let state_handle = StateHandle::new(
    niri.ipc_server.as_ref().unwrap().event_stream_state.clone(),
    backend.ipc_outputs(),
);
// Store in Niri for compositor updates
niri.state_handle = Some(state_handle.clone());
// Pass to Lua runtime
if let Some(lua_runtime) = &mut niri.lua_runtime {
    lua_runtime.set_state_handle(state_handle);
}
```
#### 2. Cursor Position Updates (src/input/mod.rs)
```rust
// In on_pointer_motion() after updating pointer position
if let Some(handle) = &self.niri.state_handle {
    let output_name = self.niri.output_under_cursor()
        .map(|o| o.name().to_string())
        .unwrap_or_default();
    handle.set_cursor_position(Some(CursorPosition {
        x: pos.x,
        y: pos.y,
        output: output_name,
    }));
}
```
#### 3. Focus Mode Updates (src/niri.rs)
```rust
// In update_keyboard_focus() or similar focus-changing code
if let Some(handle) = &self.state_handle {
    let mode = if self.is_locked { FocusMode::Locked }
               else if self.overview_open { FocusMode::Overview }
               else if self.layer_shell_focused { FocusMode::LayerShell }
               else { FocusMode::Normal };
    handle.set_focus_mode(mode);
}
```
#### 4. Reserved Space Updates (src/handlers/layer_shell.rs)
```rust
// After layer shell exclusive zone changes
if let Some(handle) = &state.niri.state_handle {
    let reserved = calculate_reserved_space(output);
    handle.set_reserved_space(&output_name, reserved);
}
```
#### 5. Lua Runtime Registration (niri-lua/src/runtime_api.rs)
```rust
pub fn register_runtime_api(lua: &Lua) -> Result<()> {
    let niri: Table = get_or_create_niri_table(lua)?;
    let state_table = lua.create_table()?;
    // windows() - collection query
    state_table.set("windows", lua.create_function(|lua, ()| {
        let handle = get_state_handle(lua)?;
        let windows = handle.windows();
        windows_to_lua(lua, &windows)
    })?)?;
    // window(id) - targeted query (NEW)
    state_table.set("window", lua.create_function(|lua, id: u64| {
        let handle = get_state_handle(lua)?;
        match handle.window(id) {
            Some(w) => Ok(Some(window_to_lua(lua, &w)?)),
            None => Ok(None),
        }
    })?)?;
    // ... similar for all other queries
    niri.set("state", state_table)?;
    Ok(())
}
fn get_state_handle(lua: &Lua) -> LuaResult<std::cell::Ref<'_, StateHandle>> {
    lua.app_data_ref::<StateHandle>()
        .ok_or_else(|| mlua::Error::external(
            "StateHandle not initialized. This is a bug in niri."
        ))
}
```
### Code Removal
The following code will be **removed** after StateHandle is working:
| File | Code to Remove |
|------|----------------|
| `niri-lua/src/runtime_api.rs` | `SCOPED_STATE_GLOBAL_KEY` constant |
| `niri-lua/src/runtime_api.rs` | `SCOPED_STATE_ACTIVE` thread-local |
| `niri-lua/src/runtime_api.rs` | `set_scoped_state_active()` function |
| `niri-lua/src/runtime_api.rs` | `is_scoped_state_active()` function |
| `niri-lua/src/runtime_api.rs` | `get_scoped_state_table()` function |
| `niri-lua/src/runtime_api.rs` | `with_scoped_state()` function (~150 lines) |
| `src/lua_event_hooks.rs` | `emit_with_scoped_state()` function (~145 lines) |
| `src/lua_event_hooks.rs` | `emit_with_niri_context()` function (~35 lines) |
| `src/lua_event_hooks.rs` | Scoped function creation in both helpers |
### Simplified Event Emission
After refactor, event emission becomes simpler:
```rust
// BEFORE: Complex scoped state setup
fn emit_with_scoped_state<F>(state: &State, event_name: &str, create_data: F)
where F: FnOnce(&Lua) -> LuaResult<LuaValue>
{
    // ~145 lines of scope setup, function creation, flag management
}
// AFTER: Simple event emission
fn emit_event<F>(state: &State, event_name: &str, create_data: F)
where F: FnOnce(&Lua) -> LuaResult<LuaValue>
{
    if let Some(lua_runtime) = &state.niri.lua_runtime {
        if let Some(event_system) = &lua_runtime.event_system {
            let lua = lua_runtime.inner();
            if let Err(e) = create_data(lua)
                .and_then(|data| event_system.emit(lua, event_name, data))
            {
                debug!("Failed to emit {} event: {}", event_name, e);
            }
        }
    }
}
```
## Features
### Feature: StateHandle Core Implementation
**Brief Reference**: Core abstraction for always-live state access
**Phase**: 1
**Complexity**: M
**Dependencies**: None
**Acceptance Criteria**:
```gherkin
GIVEN a StateHandle is created with valid shared state references
WHEN the StateHandle is cloned
THEN both instances share the same underlying state
GIVEN a StateHandle stored in Lua app_data
WHEN Lua code calls niri.state.windows()
THEN the query succeeds without requiring scoped context
GIVEN windows exist in EventStreamState
WHEN StateHandle.windows() is called
THEN all windows are returned as a Vec<Window>
GIVEN a window is added to EventStreamState
WHEN StateHandle.windows() is called again
THEN the new window is included (live data)
```
### Feature: Targeted State Queries
**Brief Reference**: O(1) lookups for individual items
**Phase**: 1
**Complexity**: S
**Dependencies**: StateHandle Core
**Acceptance Criteria**:
```gherkin
GIVEN a window with ID 123 exists
WHEN niri.state.window(123) is called
THEN the window data is returned
GIVEN no window with ID 999 exists
WHEN niri.state.window(999) is called
THEN nil is returned (no error)
GIVEN a workspace named "dev" exists
WHEN niri.state.workspace("dev") is called
THEN the workspace data is returned
GIVEN a workspace with index 2 exists
WHEN niri.state.workspace(2) is called
THEN the workspace data is returned
GIVEN an output named "DP-1" exists
WHEN niri.state.output("DP-1") is called
THEN the output data is returned
```
### Feature: Live Cursor/Focus/Reserved Updates
**Brief Reference**: Real-time updates for compositor state
**Phase**: 2
**Complexity**: M
**Dependencies**: StateHandle Core
**Acceptance Criteria**:
```gherkin
GIVEN the pointer moves to position (100, 200) on output "DP-1"
WHEN StateHandle.set_cursor_position() is called
AND Lua queries niri.state.cursor_position()
THEN {x=100, y=200, output="DP-1"} is returned
GIVEN the compositor enters overview mode
WHEN StateHandle.set_focus_mode(Overview) is called
AND Lua queries niri.state.focus_mode()
THEN "overview" is returned
GIVEN a layer shell surface claims 30px top exclusive zone on "DP-1"
WHEN StateHandle.set_reserved_space() is called
AND Lua queries niri.state.reserved_space("DP-1")
THEN {top=30, bottom=0, left=0, right=0} is returned
```
### Feature: Event Emission Simplification
**Brief Reference**: Remove scoped state setup from event emission
**Phase**: 3
**Complexity**: M
**Dependencies**: StateHandle Core, All queries working
**Acceptance Criteria**:
```gherkin
GIVEN an event handler is registered for "window:open"
WHEN a window opens and the event is emitted
THEN the event handler can call niri.state.windows()
AND the handler receives live state (including the new window)
GIVEN a timer callback is registered
WHEN the timer fires
THEN the callback can call niri.state.* functions
AND all queries return live data
GIVEN Lua code is executed via IPC
WHEN the code calls niri.state.windows()
THEN the query succeeds with live data
```
### Feature: Legacy Code Removal
**Brief Reference**: Remove scoped state infrastructure
**Phase**: 4
**Complexity**: S
**Dependencies**: All other features complete, tests passing
**Acceptance Criteria**:
```gherkin
GIVEN the StateHandle refactor is complete
WHEN SCOPED_STATE_ACTIVE is removed
THEN all tests still pass
GIVEN with_scoped_state() is removed
WHEN timer callbacks execute
THEN they still have access to state via StateHandle
GIVEN emit_with_scoped_state() is removed
WHEN events are emitted
THEN event handlers still receive live state access
```
## Implementation Phases
### Phase 1: StateHandle Foundation
**Goal**: Create StateHandle struct and basic query infrastructure
**Features**:
- StateHandle Core Implementation
- Targeted State Queries
**Done Criteria**:
- [ ] StateHandle struct created in niri-lua/src/state_handle.rs
- [ ] All collection queries implemented (windows, workspaces, outputs, keyboard_layouts)
- [ ] All targeted queries implemented (window, workspace, output)
- [ ] StateHandle registered in Lua app_data
- [ ] niri.state.* functions query StateHandle instead of scoped table
- [ ] Unit tests pass for all query methods
- [ ] Integration tests pass for state access from multiple contexts
### Phase 2: Live Compositor State
**Goal**: Add cursor, focus, and reserved space to StateHandle
**Features**:
- Live Cursor/Focus/Reserved Updates
**Done Criteria**:
- [ ] cursor_position, focus_mode, reserved_spaces fields added to StateHandle
- [ ] Update calls added to compositor at appropriate points
- [ ] niri.state.cursor_position() queries StateHandle
- [ ] niri.state.focus_mode() queries StateHandle
- [ ] niri.state.reserved_space() queries StateHandle
- [ ] All existing tests still pass
- [ ] New tests for live updates pass
### Phase 3: Event Emission Simplification
**Goal**: Remove scoped state creation from event emission
**Features**:
- Event Emission Simplification
**Done Criteria**:
- [ ] New simplified emit_event() helper created
- [ ] StateLuaEvents methods use simplified emission
- [ ] NiriLuaEvents methods use simplified emission
- [ ] Event handlers can still access niri.state.*
- [ ] All event-related tests pass
### Phase 4: Legacy Cleanup
**Goal**: Remove old scoped state infrastructure
**Features**:
- Legacy Code Removal
**Done Criteria**:
- [ ] SCOPED_STATE_ACTIVE removed
- [ ] SCOPED_STATE_GLOBAL_KEY removed
- [ ] with_scoped_state() removed
- [ ] emit_with_scoped_state() removed
- [ ] emit_with_niri_context() removed
- [ ] All tests pass
- [ ] No functionality regression
## Test Strategy
### Unit Tests (niri-lua/src/state_handle.rs)
| Test | Description |
|------|-------------|
| `test_statehandle_creation` | StateHandle::new() with valid inputs |
| `test_statehandle_clone_shares_state` | Cloned handles share underlying state |
| `test_windows_returns_all` | windows() returns all windows from EventStreamState |
| `test_window_by_id_found` | window(id) returns Some for existing window |
| `test_window_by_id_not_found` | window(id) returns None for missing window |
| `test_workspace_by_id` | workspace_by_id() O(1) lookup |
| `test_workspace_by_name` | workspace_by_name() finds by name |
| `test_workspace_by_idx` | workspace_by_idx() finds by index |
| `test_output_by_name` | output_by_name() finds by name |
| `test_focused_window` | focused_window() returns focused or None |
| `test_cursor_position_update` | set/get cursor position |
| `test_focus_mode_update` | set/get focus mode |
| `test_reserved_space_update` | set/get/remove reserved space |
### Integration Tests (niri-lua/tests/)
| Test | Description |
|------|-------------|
| `test_state_access_in_event_handler` | niri.state.* works in event callbacks |
| `test_state_access_in_timer` | niri.state.* works in timer callbacks |
| `test_state_access_in_repl` | niri.state.* works in REPL execution |
| `test_state_access_in_ipc` | niri.state.* works in IPC Lua execution |
| `test_live_data_updates` | Changes to EventStreamState reflected in queries |
| `test_concurrent_queries` | Multiple queries don't deadlock |
| `test_backward_compatibility` | Existing Lua scripts work unchanged |
### Fixtures Needed
```rust
// Mock StateHandle for testing
fn create_test_state_handle() -> StateHandle {
    let event_stream_state = Rc::new(RefCell::new(EventStreamState::default()));
    let output_map = Arc::new(Mutex::new(HashMap::new()));
    StateHandle::new(event_stream_state, output_map)
}
// Mock window for testing
fn make_test_window(id: u64, title: &str, is_focused: bool) -> Window {
    Window {
        id,
        title: Some(title.to_string()),
        app_id: Some("test-app".to_string()),
        is_focused,
        // ... other fields
    }
}
// Mock workspace for testing
fn make_test_workspace(id: u64, idx: u8, name: Option<&str>) -> Workspace {
    Workspace {
        id,
        idx,
        name: name.map(String::from),
        // ... other fields
    }
}
```
### Coverage Targets
| Module | Target |
|--------|--------|
| state_handle.rs | 90% |
| runtime_api.rs (modified) | 85% |
| lua_event_hooks.rs (modified) | 80% |
## Pseudocode
### StateHandle Query Flow
```
FUNCTION lua_state_windows():
    handle = lua.app_data_ref::<StateHandle>()
    IF handle is None:
        RETURN Error("StateHandle not initialized")

    state = handle.event_stream_state.borrow()
    windows = state.windows.windows.values().cloned().collect()

    result_table = lua.create_table()
    FOR EACH (index, window) IN windows.enumerate():
        window_table = window_to_lua(window)
        result_table.set(index + 1, window_table)

    RETURN result_table
FUNCTION lua_state_window(id: u64):
    handle = lua.app_data_ref::<StateHandle>()
    IF handle is None:
        RETURN Error("StateHandle not initialized")

    state = handle.event_stream_state.borrow()
    window = state.windows.windows.get(id)

    IF window is Some:
        RETURN window_to_lua(window)
    ELSE:
        RETURN Nil
```
### Compositor Update Flow
```
FUNCTION on_pointer_motion(position, output):
    // ... existing pointer handling ...

    IF state_handle is Some:
        state_handle.set_cursor_position(Some(CursorPosition {
            x: position.x,
            y: position.y,
            output: output.name,
        }))
FUNCTION update_focus_state():
    // ... existing focus handling ...

    IF state_handle is Some:
        mode = MATCH current_state:
            locked => FocusMode::Locked
            overview_open => FocusMode::Overview
            layer_shell_focused => FocusMode::LayerShell
            _ => FocusMode::Normal

        state_handle.set_focus_mode(mode)
```
### Simplified Event Emission
```
FUNCTION emit_event(state, event_name, create_data):
    lua_runtime = state.niri.lua_runtime
    IF lua_runtime is None:
        RETURN

    event_system = lua_runtime.event_system
    IF event_system is None:
        RETURN

    lua = lua_runtime.inner()

    TRY:
        data = create_data(lua)
        event_system.emit(lua, event_name, data)
    CATCH error:
        debug!("Failed to emit {} event: {}", event_name, error)
```
## Migration Guide
### For Existing Lua Scripts
**No changes required.** The `niri.state.*` API surface remains identical.
### For Compositor Developers
1. Access StateHandle via `state.niri.state_handle`
2. Call update methods when relevant state changes:
   - `set_cursor_position()` on pointer motion
   - `set_focus_mode()` on focus changes
   - `set_reserved_space()` on layer shell changes
### Breaking Changes
None for Lua API consumers. Internal refactor only.
## Files Changed
| File | Change Type | Description |
|------|-------------|-------------|
| `niri-lua/src/state_handle.rs` | **NEW** | StateHandle struct and methods |
| `niri-lua/src/lib.rs` | MODIFY | Export StateHandle, remove scoped state exports |
| `niri-lua/src/runtime_api.rs` | MODIFY | Simplify to query StateHandle, remove 150+ lines |
| `niri-lua/src/runtime.rs` | MODIFY | Add set_state_handle() method |
| `src/lua_event_hooks.rs` | MODIFY | Simplify event emission, remove 180+ lines |
| `src/niri.rs` | MODIFY | Create StateHandle, add to Niri struct |
| `src/input/mod.rs` | MODIFY | Update cursor position on motion |
| `src/handlers/layer_shell.rs` | MODIFY | Update reserved space on changes |
## Risks and Mitigations
| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Deadlock from Arc<Mutex<>> | Low | High | Use try_lock with fallback, document lock ordering |
| Performance regression | Low | Medium | Benchmark before/after, lazy evaluation |
| Backward compatibility break | Low | High | Extensive integration tests with existing scripts |
| Incomplete state updates | Medium | Medium | Audit all state mutation points in compositor |
## Success Metrics
1. **All existing tests pass** - No functionality regression
2. **New targeted queries work** - window(id), workspace(ref), output(name)
3. **State access works everywhere** - Events, timers, REPL, IPC
4. **Code reduction** - ~300+ lines removed from scoped state infrastructure
5. **Performance parity** - No measurable slowdown in state queries
