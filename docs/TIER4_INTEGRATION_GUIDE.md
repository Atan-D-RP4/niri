# Phase 2: Event System Integration Guide

## Overview

This document describes how to integrate the Tier 4 Event System (Foundation) with Niri's compositor core to emit real events to Lua scripts.

## Architecture

```
Niri Core Events
    ↓
Event Emission Points (handlers, layout, backend)
    ↓
EventSystem (Thread-safe wrapper)
    ↓
Lua Event Handlers (niri.on/once/off)
    ↓
User Lua Scripts
```

## Event Categories

### 1. Window Events

**Location**: `src/handlers/xdg_shell.rs`

**Events to emit**:

- **window:open** - Window surface created and configured
  - Triggers: `new_toplevel()` → window maps → `Initial configure sent`
  - Data: `WindowEventData::Open { window }`
  - Lua: `niri.on("window:open", function(event) ... end)`

- **window:close** - Window is being destroyed
  - Triggers: `toplevel_destroyed()`
  - Data: `WindowEventData::Close { window }`
  - Lua: `niri.on("window:close", function(event) ... end)`

- **window:focus** - Window gained keyboard focus
  - Triggers: `update_keyboard_focus()` or layout focus change
  - Data: `WindowEventData::Focus { window }`
  - Lua: `niri.on("window:focus", function(event) ... end)`

- **window:blur** - Window lost keyboard focus
  - Triggers: When focus moves away
  - Data: `WindowEventData::Blur { window }`
  - Lua: `niri.on("window:blur", function(event) ... end)`

### 2. Workspace Events

**Location**: `src/layout/mod.rs`

**Events to emit**:

- **workspace:activate** - Workspace becomes active on its output
  - Triggers: `activate_workspace()` or workspace switch
  - Data: `WorkspaceEventData::Activate { workspace, output }`
  - Lua: `niri.on("workspace:activate", function(event) ... end)`

- **workspace:deactivate** - Workspace is no longer active
  - Triggers: When workspace loses focus
  - Data: `WorkspaceEventData::Deactivate { workspace }`
  - Lua: `niri.on("workspace:deactivate", function(event) ... end)`

### 3. Monitor Events

**Location**: `src/backend/mod.rs` or `src/niri.rs`

**Events to emit**:

- **monitor:connect** - Monitor/output connected
  - Triggers: Output added to global space
  - Data: `MonitorEventData::Connect { output }`
  - Lua: `niri.on("monitor:connect", function(event) ... end)`

- **monitor:disconnect** - Monitor/output disconnected
  - Triggers: Output removed from global space
  - Data: `MonitorEventData::Disconnect { output }`
  - Lua: `niri.on("monitor:disconnect", function(event) ... end)`

### 4. Layout Events

**Location**: `src/layout/workspace.rs` or `src/layout/mod.rs`

**Events to emit**:

- **layout:mode_changed** - Switched between tiling and floating
  - Triggers: `toggle_window_floating()` mode change
  - Data: `LayoutEventData::ModeChanged { is_floating }`
  - Lua: `niri.on("layout:mode_changed", function(event) ... end)`

- **layout:window_added** - Window added to layout
  - Triggers: `add_window()` or window map
  - Data: `LayoutEventData::WindowAdded { window }`
  - Lua: `niri.on("layout:window_added", function(event) ... end)`

- **layout:window_removed** - Window removed from layout
  - Triggers: `remove_window()` or window close
  - Data: `LayoutEventData::WindowRemoved { window }`
  - Lua: `niri.on("layout:window_removed", function(event) ... end)`

## Integration Steps

### Step 1: Create Event Emission Module

Create `src/lua_event_hooks.rs`:

```rust
//! Event emission hooks for Lua integration.
//!
//! This module provides functions to emit events from Niri core to Lua scripts
//! through the event system. It acts as a bridge between the compositor and Lua.

use niri_lua::{EventData, MonitorEventData, WindowEventData, WorkspaceEventData, LayoutEventData, SharedEventHandlers};
use niri_ipc::{Window, Workspace, Output};
use anyhow::Result;

/// Emit a window open event.
pub fn emit_window_open(handlers: &SharedEventHandlers, window: Window) -> Result<()> {
    let event = EventData::Window(WindowEventData::Open { window });
    handlers.emit("window:open", event)?;
    Ok(())
}

/// Emit a window close event.
pub fn emit_window_close(handlers: &SharedEventHandlers, window: Window) -> Result<()> {
    let event = EventData::Window(WindowEventData::Close { window });
    handlers.emit("window:close", event)?;
    Ok(())
}

/// Emit a window focus event.
pub fn emit_window_focus(handlers: &SharedEventHandlers, window: Window) -> Result<()> {
    let event = EventData::Window(WindowEventData::Focus { window });
    handlers.emit("window:focus", event)?;
    Ok(())
}

// ... similar for other events
```

### Step 2: Update EventSystem to Support Emission

Modify `niri-lua/src/event_system.rs`:

```rust
impl EventSystem {
    /// Emit an event to all registered handlers
    pub fn emit(&self, event_name: &str, event_data: EventData) -> Result<()> {
        let handlers = self.handlers.lock();
        
        // Convert event data to Lua table
        let lua_event = event_data.to_lua(&self.lua)?;
        
        // Call all handlers for this event
        handlers.emit(event_name, lua_event)?;
        
        Ok(())
    }
}
```

### Step 3: Add SharedEventHandlers to State

Modify `src/niri.rs`:

```rust
pub struct Niri {
    // ... existing fields ...
    
    /// Lua event system for emitting events to scripts
    pub lua_event_handlers: Option<niri_lua::SharedEventHandlers>,
}
```

### Step 4: Wire Event Emission in Handlers

**Window events in `src/handlers/xdg_shell.rs`**:

```rust
fn toplevel_destroyed(&mut self, surface: ToplevelSurface) {
    // ... existing code ...
    
    // Emit window close event
    if let Some(handlers) = &self.niri.lua_event_handlers {
        let window = // ... get window data from IPC ...
        let _ = lua_event_hooks::emit_window_close(handlers, window);
    }
    
    // ... rest of function ...
}
```

**Workspace events in `src/layout/mod.rs`**:

```rust
pub fn activate_workspace(&mut self, workspace: &Workspace) {
    // ... existing code ...
    
    // Emit workspace activate event
    if let Some(handlers) = &self.niri.lua_event_handlers {
        let ws_data = // ... convert to Workspace ...
        let _ = lua_event_hooks::emit_workspace_activate(handlers, ws_data);
    }
}
```

### Step 5: Monitor Output Changes

**Monitor events in `src/niri.rs`**:

```rust
fn connector_connected(&mut self, connector: Connector) {
    // ... existing code ...
    
    // Emit monitor connect event
    if let Some(handlers) = &self.niri.lua_event_handlers {
        let output = // ... convert output ...
        let _ = lua_event_hooks::emit_monitor_connect(handlers, output);
    }
}

fn connector_disconnected(&mut self, connector: Connector) {
    // ... existing code ...
    
    // Emit monitor disconnect event  
    if let Some(handlers) = &self.niri.lua_event_handlers {
        let output = // ... convert output ...
        let _ = lua_event_hooks::emit_monitor_disconnect(handlers, output);
    }
}
```

## Event Data Conversion

Use existing IPC converters in `niri-lua/src/ipc_bridge.rs`:

```rust
// Window conversion
let window = ipc_bridge::window_to_lua(&lua, &window)?;

// Workspace conversion  
let workspace = ipc_bridge::workspace_to_lua(&lua, &workspace)?;

// Output conversion
let output = ipc_bridge::output_to_lua(&lua, &output)?;
```

These already handle converting Niri's internal types to Lua tables with proper structure.

## Lua Event Handler Example

```lua
-- Listen for window open events
niri.on("window:open", function(event)
    print("Window opened:", event.window.title)
    print("App ID:", event.window.app_id)
end)

-- Listen for workspace activation
niri.on("workspace:activate", function(event)
    print("Workspace activated:", event.workspace.name)
    print("On output:", event.output.name)
end)

-- One-time monitor connection handler
niri.once("monitor:connect", function(event)
    print("First monitor connected:", event.output.name)
    -- Re-configure layout, load presets, etc.
end)

-- Monitor disconnection
niri.on("monitor:disconnect", function(event)
    print("Monitor disconnected:", event.output.name)
end)
```

## Testing Strategy

### Unit Tests

- Test each event emission function in isolation
- Verify event data conversion to Lua tables
- Test error handling and recovery

### Integration Tests

- Create test scenario with mock windows/workspaces
- Emit events and verify Lua handlers are called
- Verify event data is accessible in Lua
- Test handler cleanup and removal

### End-to-End Tests

- Run Niri with Lua config that listens to events
- Manually trigger events (open window, switch workspace, etc.)
- Verify Lua scripts respond correctly

## Performance Considerations

1. **Handler Isolation**: Errors in one handler don't crash others
2. **Thread Safety**: All emission goes through Arc<Mutex>
3. **Async Safety**: Events can be emitted from any thread (wrapped in mutex)
4. **Memory**: Event tables are garbage collected after handler execution
5. **Throughput**: Batch related events where possible to reduce overhead

## Error Handling

- Log handler errors without panicking
- Continue with next handler if one fails
- Return Result from emit functions for caller visibility
- Track failed handler counts for debugging

## Future Enhancements

1. **Event Filtering**: Let Lua pre-filter events before handler execution
2. **Event Priority**: Priority-based handler execution
3. **Event Chaining**: Handlers can emit events that trigger other handlers
4. **Event History**: Keep recent events for introspection
5. **Metrics**: Track event emission counts, handler execution times
6. **Async Handlers**: Support async Lua handlers with futures

## Migration Path

Phase 2 can be deployed incrementally:

1. Implement core window events first (open, close, focus)
2. Add workspace events
3. Add monitor events
4. Document event patterns
5. Add layout events and refinements
6. User feedback and iteration

This phased approach allows testing each event category independently and gathering user feedback before moving to the next phase.
