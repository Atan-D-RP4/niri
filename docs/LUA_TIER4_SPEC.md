# Tier 4 Specification: Event Handling System

**Status:** ✅ **IMPLEMENTED**  
**Duration:** Weeks 7-8  
**Estimated LOC:** 300-350 Rust + 150 documentation  
**Complexity:** Very High (Niri core integration required)

---

## Implementation Status

### ✅ Completed

**Core Infrastructure:**
- `niri-lua/src/event_system.rs` - Event API registration (`niri.on()`, `niri.once()`, `niri.off()`)
- `niri-lua/src/event_handlers.rs` - Handler management with error isolation
- `niri-lua/src/event_data.rs` - Event data structures for Lua
- `niri-lua/src/event_emitter.rs` - Event emission utilities
- `src/lua_event_hooks.rs` - Emit helper functions called from compositor

**Implemented Events (11 total):**
| Event | Description | Emission Location |
|-------|-------------|-------------------|
| `window:open` | Window created | `src/handlers/xdg_shell.rs` |
| `window:close` | Window destroyed | `src/handlers/xdg_shell.rs` |
| `window:focus` | Window received focus | `src/niri.rs` |
| `window:blur` | Window lost focus | `src/niri.rs` |
| `workspace:activate` | Workspace became active | `src/handlers/mod.rs` |
| `workspace:deactivate` | Workspace became inactive | `src/handlers/mod.rs` |
| `monitor:connect` | Monitor connected | `src/backend/tty.rs` |
| `monitor:disconnect` | Monitor disconnected | `src/backend/tty.rs` |
| `layout:mode_changed` | Tiling/floating toggle | `src/input/mod.rs` (5 locations) |
| `layout:window_added` | Window added to layout | `src/handlers/compositor.rs` |
| `layout:window_removed` | Window removed from layout | `src/handlers/xdg_shell.rs` |

**Documentation & Examples:**
- `docs/LUA_EVENT_HOOKS.md` - Comprehensive event system guide
- `examples/event_system_demo.lua` - Working demonstration of all events

### ⏳ Not Yet Implemented (Future Enhancement)

These events from the original spec are not yet implemented:
- `window:move` - Window moved to different workspace
- `window:resize` - Window resized
- `window:fullscreen` - Fullscreen state changed
- `window:floating` - Floating state changed (partially covered by `layout:mode_changed`)
- `window:title-changed` - Window title changed
- `workspace:create` - Workspace created
- `workspace:destroy` - Workspace destroyed
- `monitor:focus` - Monitor gained focus
- `monitor:configuration-changed` - Monitor resolution/scale changed
- `layout:column-changed` - Column count changed

---

## Overview

Tier 4 completes the event system by **integrating with Niri core** to fire events at appropriate times. Users can:
- React to window open/close events
- Respond to workspace switching
- Handle monitor connection/disconnection
- Implement custom automation logic
- Build interactive plugins

This tier is critical because it enables truly reactive, event-driven Lua programming.

---

## Architecture

```
Event System Infrastructure:
  - EventEmitter holds handler registrations
  - Event types with structured data
  - Handler error isolation (errors don't crash Niri)
  - Async delivery (non-blocking)
  
Event Integration Points in Niri Core:
  - window open/close in handlers/xdg_shell.rs
  - workspace switching in layout/mod.rs
  - monitor connection in backend/mod.rs
  - focus changes throughout
  - input events in input/mod.rs
  
Event Data Structures:
  - WindowEvent { window, old_state, new_state, ... }
  - WorkspaceEvent { workspace, action, ... }
  - MonitorEvent { monitor, action, ... }
  - InputEvent { key, modifiers, ... }
```

---

## Detailed Specifications

### 1. Event Types & Definitions

#### Window Events

```rust
#[derive(Debug, Clone)]
pub enum WindowEventType {
    /// Window created and mapped to workspace
    Open {
        window: LuaWindow,
    },
    /// Window is being destroyed
    Close {
        window: LuaWindow,
    },
    /// Window gained focus
    Focus {
        window: LuaWindow,
        previous_window: Option<LuaWindow>,
    },
    /// Window lost focus
    Blur {
        window: LuaWindow,
        next_window: Option<LuaWindow>,
    },
    /// Window moved to different workspace
    Move {
        window: LuaWindow,
        from_workspace: u64,
        to_workspace: u64,
    },
    /// Window resized (tiling or floating)
    Resize {
        window: LuaWindow,
        old_geometry: (i32, i32, i32, i32),
        new_geometry: (i32, i32, i32, i32),
    },
    /// Window entered fullscreen mode
    Fullscreen {
        window: LuaWindow,
        is_fullscreen: bool,
    },
    /// Window floating state changed
    FloatingChanged {
        window: LuaWindow,
        is_floating: bool,
    },
    /// Window title changed
    TitleChanged {
        window: LuaWindow,
        old_title: String,
        new_title: String,
    },
    /// App ID changed (shouldn't happen normally)
    AppIdChanged {
        window: LuaWindow,
        old_app_id: String,
        new_app_id: String,
    },
}
```

#### Workspace Events

```rust
#[derive(Debug, Clone)]
pub enum WorkspaceEventType {
    /// Workspace became active
    Activate {
        workspace: LuaWorkspace,
        previous_workspace: Option<LuaWorkspace>,
    },
    /// Workspace is no longer active
    Deactivate {
        workspace: LuaWorkspace,
    },
    /// Workspace created (or hidden)
    Create {
        workspace: LuaWorkspace,
    },
    /// Workspace destroyed
    Destroy {
        workspace: LuaWorkspace,
    },
    /// Window count changed
    WindowCountChanged {
        workspace: LuaWorkspace,
        old_count: u32,
        new_count: u32,
    },
}
```

#### Monitor Events

```rust
#[derive(Debug, Clone)]
pub enum MonitorEventType {
    /// Monitor connected
    Connect {
        monitor: LuaMonitor,
    },
    /// Monitor disconnected
    Disconnect {
        monitor: LuaMonitor,
    },
    /// Monitor became active (gained focus)
    Focus {
        monitor: LuaMonitor,
        previous_monitor: Option<LuaMonitor>,
    },
    /// Monitor resolution or refresh rate changed
    ConfigurationChanged {
        monitor: LuaMonitor,
        old_scale: f32,
        new_scale: f32,
        old_refresh_rate: f32,
        new_refresh_rate: f32,
    },
}
```

#### Layout Events

```rust
#[derive(Debug, Clone)]
pub enum LayoutEventType {
    /// Layout switched between tiling and floating
    LayoutSwitch {
        from_layout: String,
        to_layout: String,
    },
    /// Column added or removed
    ColumnCountChanged {
        old_count: u32,
        new_count: u32,
    },
}
```

#### Input Events (Tier 4 Partial, Full in Tier 5)

```rust
#[derive(Debug, Clone)]
pub enum InputEventType {
    /// Custom gesture event (from Lua)
    GestureCustom {
        gesture_name: String,
        data: HashMap<String, String>,
    },
    /// Key was pressed and not handled
    KeyUnhandled {
        key: String,
        modifiers: Vec<String>,
    },
}
```

### 2. Event Handler Registration

#### Lua API

```lua
-- Register event handler (called every time event fires)
niri.on("window:open", function(data)
    print("Window opened: " .. data:title())
end)

-- One-time handler (called once, then removed)
niri.once("window:close", function(data)
    print("A window closed")
end)

-- Unregister specific handler
local my_handler = function(data) end
niri.on("workspace:activate", my_handler)
niri.off("workspace:activate", my_handler)

-- Listen to multiple events
for _, event in ipairs({
    "window:open",
    "window:close",
    "window:focus",
}) do
    niri.on(event, function(data)
        niri.log("Window event: " .. event)
    end)
end
```

#### Handler Storage

```rust
pub struct EventHandler {
    event_type: String,
    callback: LuaFunction,
    once: bool,  // true for one-time handlers
}

pub struct EventHandlers {
    handlers: HashMap<String, Vec<EventHandler>>,
    lua: Arc<Lua>,
}

impl EventHandlers {
    pub fn register(&mut self, event_type: &str, callback: LuaFunction, once: bool) {
        let handler = EventHandler {
            event_type: event_type.to_string(),
            callback,
            once,
        };
        
        self.handlers
            .entry(event_type.to_string())
            .or_insert_with(Vec::new)
            .push(handler);
    }
    
    pub fn unregister(&mut self, event_type: &str, callback: &LuaFunction) {
        // Match handler by function reference and remove
    }
    
    pub fn emit(&mut self, event_type: &str, data: LuaValue) -> anyhow::Result<()> {
        if let Some(handlers) = self.handlers.get(event_type) {
            let handlers_to_call: Vec<_> = handlers
                .iter()
                .map(|h| (h.callback.clone(), h.once))
                .collect();
            
            for (callback, once) in handlers_to_call {
                // Call handler, isolating errors
                if let Err(e) = callback.call::<_, ()>(data.clone()) {
                    eprintln!("Error in {} handler: {}", event_type, e);
                    // Don't propagate error - keep Niri running
                }
                
                // Remove one-time handlers
                if once {
                    self.unregister(event_type, &callback);
                }
            }
        }
        
        Ok(())
    }
}
```

### 3. Integration Points in Niri Core

#### Window Open Event (in `src/handlers/xdg_shell.rs`)

```rust
// When window is mapped:
pub fn handle_window_mapped(&mut self, surface: &WlSurface) {
    // ... existing code ...
    
    // NEW: Emit Lua event
    if let Some(emitter) = &mut self.lua_event_emitter {
        let window = LuaWindow::from_surface(surface);
        let event = lua.create_table()?;
        event.set("window", window)?;
        
        let _ = emitter.emit("window:open", LuaValue::Table(event));
    }
}
```

#### Window Focus Event (in focus handling code)

```rust
pub fn set_focus(&mut self, window_id: u64) {
    let old_focus = self.focused_window;
    self.focused_window = Some(window_id);
    
    // NEW: Emit Lua events
    if let Some(emitter) = &mut self.lua_event_emitter {
        let new_window = LuaWindow::from_id(window_id);
        let prev_window = old_focus.and_then(|id| LuaWindow::from_id(id));
        
        let event = lua.create_table()?;
        event.set("window", new_window)?;
        event.set("previous_window", prev_window)?;
        
        let _ = emitter.emit("window:focus", LuaValue::Table(event));
    }
}
```

#### Workspace Switch Event (in `src/layout/mod.rs`)

```rust
pub fn activate_workspace(&mut self, ws_id: u64) {
    let old_ws = self.active_workspace;
    self.active_workspace = ws_id;
    
    // NEW: Emit Lua event
    if let Some(emitter) = &mut self.lua_event_emitter {
        let workspace = LuaWorkspace::from_id(ws_id);
        let prev_workspace = old_ws.and_then(|id| LuaWorkspace::from_id(id));
        
        let event = lua.create_table()?;
        event.set("workspace", workspace)?;
        event.set("previous_workspace", prev_workspace)?;
        
        let _ = emitter.emit("workspace:activate", LuaValue::Table(event));
    }
}
```

#### Monitor Connection Event (in `src/backend/mod.rs`)

```rust
pub fn on_monitor_connected(&mut self, monitor: &Monitor) {
    // ... existing code ...
    
    // NEW: Emit Lua event
    if let Some(emitter) = &mut self.lua_event_emitter {
        let lua_monitor = LuaMonitor::from_monitor(monitor);
        
        let event = lua.create_table()?;
        event.set("monitor", lua_monitor)?;
        
        let _ = emitter.emit("monitor:connect", LuaValue::Table(event));
    }
}
```

### 4. Event Handlers Integration

#### File: `src/lua_extensions/event_handlers.rs`

```rust
pub struct EventHandlers {
    handlers: HashMap<String, Vec<LuaEventHandler>>,
}

#[derive(Clone)]
pub struct LuaEventHandler {
    callback: LuaFunction,
    once: bool,
}

impl EventHandlers {
    pub fn new() -> Self {
        EventHandlers {
            handlers: HashMap::new(),
        }
    }
    
    pub fn register_handler(
        &mut self,
        event_type: &str,
        callback: LuaFunction,
        once: bool,
    ) -> anyhow::Result<()> {
        let handler = LuaEventHandler { callback, once };
        self.handlers
            .entry(event_type.to_string())
            .or_insert_with(Vec::new)
            .push(handler);
        Ok(())
    }
    
    pub fn emit_event(
        &mut self,
        event_type: &str,
        event_data: LuaValue,
    ) -> anyhow::Result<()> {
        if let Some(handlers) = self.handlers.get(event_type).cloned() {
            for (i, handler) in handlers.iter().enumerate() {
                // Call handler with error isolation
                match handler.callback.call::<_, ()>(event_data.clone()) {
                    Ok(_) => {
                        // Remove one-time handlers
                        if handler.once {
                            if let Some(handlers) = self.handlers.get_mut(event_type) {
                                handlers.remove(i);
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!(
                            "Error in {} event handler: {}",
                            event_type, e
                        );
                        // Continue to next handler
                    }
                }
            }
        }
        Ok(())
    }
}
```

### 5. Lua Registration

#### File: `src/lua_extensions/event_system.rs`

```rust
pub fn register_event_api_to_lua(
    lua: &Lua,
    handlers: Arc<Mutex<EventHandlers>>,
) -> LuaResult<()> {
    let niri_table = lua.globals().get::<_, LuaTable>("niri")?;
    
    // niri.on(event_type, callback)
    let on_fn = {
        let handlers = handlers.clone();
        lua.create_function(move |_, (event_type, callback): (String, LuaFunction)| {
            let mut h = handlers.lock().unwrap();
            h.register_handler(&event_type, callback, false)?;
            Ok(())
        })?
    };
    niri_table.set("on", on_fn)?;
    
    // niri.once(event_type, callback)
    let once_fn = {
        let handlers = handlers.clone();
        lua.create_function(move |_, (event_type, callback): (String, LuaFunction)| {
            let mut h = handlers.lock().unwrap();
            h.register_handler(&event_type, callback, true)?;
            Ok(())
        })?
    };
    niri_table.set("once", once_fn)?;
    
    // niri.off(event_type, callback) - unregister
    let off_fn = {
        let handlers = handlers.clone();
        lua.create_function(move |_, (event_type, _callback): (String, LuaFunction)| {
            // Implementation for unregistering handlers
            Ok(())
        })?
    };
    niri_table.set("off", off_fn)?;
    
    Ok(())
}
```

---

## Example Use Cases

### Auto-float Firefox Pop-ups

```lua
niri.on("window:open", function(event)
    local window = event.window
    if window:app_id() == "firefox" and window:title():match("^About") then
        niri.command("toggle-floating")
    end
end)
```

### Workspace-specific Automation

```lua
niri.on("workspace:activate", function(event)
    local ws = event.workspace
    if ws:name() == "work" then
        -- Auto-start work applications
        niri.spawn("slack")
        niri.spawn("slack-terminal")
    end
end)
```

### Monitor Connection Handler

```lua
niri.on("monitor:connect", function(event)
    local monitor = event.monitor
    niri.log("Monitor connected: " .. monitor:name())
    niri.log("Scale: " .. monitor:scale() .. "x")
    niri.log("Resolution: " .. monitor:width() .. "x" .. monitor:height())
    
    -- Could auto-configure or notify user
end)
```

### Window Lifecycle Tracking

```lua
local open_windows = {}

niri.on("window:open", function(event)
    local window = event.window
    open_windows[window:id()] = window
end)

niri.on("window:close", function(event)
    local window = event.window
    open_windows[window:id()] = nil
end)

-- Can be queried later
function get_open_window_count()
    local count = 0
    for _ in pairs(open_windows) do
        count = count + 1
    end
    return count
end
```

---

## File Structure Summary

**Actual Implementation Files:**
- `niri-lua/src/event_system.rs` (~150 lines) - Event API registration
- `niri-lua/src/event_handlers.rs` (~100 lines) - Handler management
- `niri-lua/src/event_data.rs` (~200 lines) - Event data structures
- `niri-lua/src/event_emitter.rs` (~50 lines) - Emission utilities
- `src/lua_event_hooks.rs` (~150 lines) - Emit helpers for compositor

**Modified Files:**
- `niri-lua/src/lib.rs` - Module exports
- `niri-lua/src/runtime.rs` - EventSystem initialization
- `niri-lua/src/config.rs` - Auto-initialize event system
- `src/handlers/xdg_shell.rs` - Window open/close events
- `src/handlers/compositor.rs` - Layout window_added events
- `src/handlers/mod.rs` - Workspace activate/deactivate events
- `src/niri.rs` - Window focus/blur events
- `src/backend/tty.rs` - Monitor connect/disconnect events
- `src/input/mod.rs` - Layout mode_changed events

---

## Testing Strategy

### Unit Tests

```rust
#[test]
fn test_event_handler_registration() {
    let mut handlers = EventHandlers::new();
    // Register handler and verify it's stored
}

#[test]
fn test_event_emission() {
    let mut handlers = EventHandlers::new();
    // Register handler, emit event, verify callback called
}

#[test]
fn test_one_time_handler() {
    let mut handlers = EventHandlers::new();
    // Register once handler, emit twice, verify called once
}

#[test]
fn test_handler_error_isolation() {
    let mut handlers = EventHandlers::new();
    // Add handler that panics, emit, verify other handlers still called
}
```

### Integration Tests

```rust
#[test]
fn test_window_open_event() {
    // Mock window opening, verify event fires
}

#[test]
fn test_workspace_switch_event() {
    // Mock workspace switch, verify event fires
}
```

---

## Success Criteria

✅ All core event types firing correctly (11 events implemented)  
✅ Handler registration/unregistration working (`niri.on()`, `niri.once()`, `niri.off()`)  
✅ One-time handlers work correctly  
✅ Event handler errors don't crash Niri (isolated error handling)  
✅ All niri-lua tests passing (408 tests)  
✅ Event system auto-initialized during config loading  
⏳ Additional event types (window:move, window:resize, etc.) - Future enhancement  

---

## Event Namespace Reference

```
window:open           → { window: Window }
window:close          → { window: Window }
window:focus          → { window: Window, previous_window?: Window }
window:blur           → { window: Window, next_window?: Window }
window:move           → { window: Window, from_workspace: id, to_workspace: id }
window:resize         → { window: Window, old_geometry: (x,y,w,h), new_geometry: (x,y,w,h) }
window:fullscreen     → { window: Window, is_fullscreen: bool }
window:floating       → { window: Window, is_floating: bool }
window:title-changed  → { window: Window, old_title: str, new_title: str }

workspace:activate    → { workspace: Workspace, previous_workspace?: Workspace }
workspace:deactivate  → { workspace: Workspace }
workspace:create      → { workspace: Workspace }
workspace:destroy     → { workspace: Workspace }

monitor:connect       → { monitor: Monitor }
monitor:disconnect    → { monitor: Monitor }
monitor:focus         → { monitor: Monitor, previous_monitor?: Monitor }

layout:layout-switch  → { from_layout: str, to_layout: str }
layout:column-changed → { old_count: int, new_count: int }
```

---

## References

- [Event-Driven Programming](https://en.wikipedia.org/wiki/Event-driven_architecture)
- [Neovim Autocommands](https://neovim.io/doc/user/autocmd.html)
- [AwesomeWM Signals](https://awesomewm.org/doc/topics/signals.html)
