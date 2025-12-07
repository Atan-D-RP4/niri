# Niri Lua Event System Guide

## Overview

The Niri Lua event system provides a way for Lua scripts to listen to and react to compositor events. This enables powerful automation, monitoring, and custom behaviors based on system state changes.

The event system is based on a pub-sub pattern where scripts register handlers (callbacks) for specific event types, and the compositor emits events when things happen.

> **Implementation Status:** The event infrastructure is complete, but **not all events are wired** to compositor code yet. See the [Event Wiring Status](#event-wiring-status) section for details.

> **TODO: Simplify event_emitter.rs** - The current implementation has two parallel architectures:
> a Rust `EventEmitter` struct (unused) and a Lua-based implementation via global tables.
> These should be evaluated to determine which approach is better, and the unused code pruned.

## Quick Start

```lua
niri.apply_config({})

-- Listen to window open events
niri.events:on("window:open", function(data)
    niri.utils.log("Window opened: " .. (data.title or "unnamed"))
end)

-- Listen to workspace activation
niri.events:on("workspace:activate", function(data)
    niri.utils.log("Workspace activated: " .. data.name .. " (index: " .. data.index .. ")")
end)
```

## API Reference

### niri.events:on(event_type, callback)

Register a persistent event handler. The callback will be called every time the event occurs.

**Parameters:**
- `event_type` (string): The type of event to listen for
- `callback` (function): Function to call when the event occurs

**Returns:** A handler ID (can be used to unregister later)

**Example:**
```lua
local handler_id = niri.events:on("window:focus", function(data)
    print("Window focused: " .. data.title)
end)
```

### niri.events:once(event_type, callback)

Register a one-time event handler. The callback will be called only the next time the event occurs, then automatically unregistered.

**Parameters:**
- `event_type` (string): The type of event to listen for
- `callback` (function): Function to call when the event occurs (once)

**Returns:** A handler ID

**Example:**
```lua
niri.events:once("window:open", function(data)
    niri.utils.log("First window opened!")
end)
```

### niri.events:off(event_type, handler_id)

Unregister an event handler.

**Parameters:**
- `event_type` (string): The type of event
- `handler_id` (number): The handler ID returned by `niri.events:on()` or `niri.events:once()`

**Example:**
```lua
local id = niri.events:on("window:focus", function(data)
    print("Window: " .. data.title)
end)

-- Later, unregister it:
niri.events:off("window:focus", id)
```

### niri.events:emit(event_type, data)

Emit a custom event. This allows Lua scripts to create their own events for inter-plugin communication.

**Parameters:**
- `event_type` (string): The event type to emit
- `data` (table): Data to pass to handlers

**Example:**
```lua
-- Emit a custom event
niri.events:emit("custom:my-event", { message = "Hello!", count = 42 })

-- Another script can listen for it
niri.events:on("custom:my-event", function(data)
    niri.utils.log("Received: " .. data.message)
end)
```

### niri.events:list()

Query all registered event handlers.

**Returns:** A table with `total` (number of handlers) and `events` (table of event types with handler counts)

**Example:**
```lua
local info = niri.events:list()
niri.utils.log("Total handlers: " .. info.total)
for event, count in pairs(info.events) do
    niri.utils.log("  " .. event .. ": " .. count .. " handlers")
end
```

### niri.events:clear(event_type)

Remove all handlers for a specific event type.

**Parameters:**
- `event_type` (string): The event type to clear

**Example:**
```lua
-- Remove all window:focus handlers
niri.events:clear("window:focus")
```

## Event Wiring Status

The event infrastructure is complete, but many events need to be wired to compositor code paths. Here's the current status:

| Event | Status | Notes |
|-------|--------|-------|
| `startup` | ✅ Wired | Emitted before event loop starts |
| `shutdown` | ✅ Wired | Emitted after event loop exits |
| `window:open` | ⚠️ Partial | Wired but emits placeholder data (id=0, title="window") |
| `window:close` | 🚧 TODO | Defined but not wired |
| `window:focus` | 🚧 TODO | Defined but not wired |
| `window:blur` | 🚧 TODO | Defined but not wired |
| `window:title_changed` | 🚧 TODO | Defined but not wired |
| `window:app_id_changed` | 🚧 TODO | Defined but not wired |
| `window:fullscreen` | 🚧 TODO | Defined but not wired |
| `workspace:activate` | ✅ Wired | Emitted on workspace switch |
| `workspace:deactivate` | 🚧 TODO | Defined but not wired |
| `monitor:connect` | 🚧 TODO | Defined but not wired |
| `monitor:disconnect` | 🚧 TODO | Defined but not wired |
| `layout:mode_changed` | 🚧 TODO | Defined but not wired |
| `layout:window_added` | 🚧 TODO | Defined but not wired |
| `layout:window_removed` | 🚧 TODO | Defined but not wired |
| `config:reload` | 🚧 TODO | Defined but not wired |
| `overview:open` | 🚧 TODO | Defined but not wired |
| `overview:close` | 🚧 TODO | Defined but not wired |

## Event Types

### Window Events

#### window:open
Emitted when a new window is created.

> **Status:** ⚠️ Partially wired - currently emits placeholder data. TODO: Wire with real window info.

**Event data:**
```lua
{
    id = 12345,          -- Window ID (number)
    title = "App Name"   -- Window title (string)
}
```

#### window:close
Emitted when a window is destroyed.

**Event data:**
```lua
{
    id = 12345,
    title = "App Name"
}
```

#### window:focus
Emitted when a window receives keyboard focus.

**Event data:**
```lua
{
    id = 12345,
    title = "App Name"
}
```

#### window:blur
Emitted when a window loses keyboard focus.

**Event data:**
```lua
{
    id = 12345,
    title = "App Name"
}
```

#### window:title_changed
Emitted when a window's title changes.

**Event data:**
```lua
{
    id = 12345,
    title = "New Title"
}
```

#### window:app_id_changed
Emitted when a window's app_id changes.

**Event data:**
```lua
{
    id = 12345,
    app_id = "org.example.app"
}
```

#### window:fullscreen
Emitted when a window enters or exits fullscreen mode.

**Event data:**
```lua
{
    id = 12345,
    title = "App Name",
    is_fullscreen = true  -- or false
}
```

### Workspace Events

#### workspace:activate
Emitted when a workspace becomes the active (displayed) workspace.

**Event data:**
```lua
{
    name = "1",          -- Workspace name (string)
    index = 1            -- Workspace index (number, 0-indexed)
}
```

#### workspace:deactivate
Emitted when a workspace is no longer the active workspace.

**Event data:**
```lua
{
    name = "1",
    index = 1
}
```

### Monitor Events

#### monitor:connect
Emitted when a monitor is connected.

**Event data:**
```lua
{
    name = "HDMI-1",        -- Display name (string)
    connector = "HDMI-1"    -- Connector name (string)
}
```

#### monitor:disconnect
Emitted when a monitor is disconnected.

**Event data:**
```lua
{
    name = "HDMI-1",
    connector = "HDMI-1"
}
```

### Layout Events

#### layout:mode_changed
Emitted when the layout mode changes between tiling and floating.

**Event data:**
```lua
{
    mode = "tiling"   -- "tiling" or "floating" (string)
}
```

#### layout:window_added
Emitted when a window is added to the tiling layout.

**Event data:**
```lua
{
    id = 12345   -- Window ID (number)
}
```

#### layout:window_removed
Emitted when a window is removed from the tiling layout.

**Event data:**
```lua
{
    id = 12345   -- Window ID (number)
}
```

### System Events

#### config:reload
Emitted when the configuration is reloaded.

**Event data:**
```lua
{
    success = true  -- Whether reload succeeded (boolean)
}
```

#### overview:open
Emitted when the overview mode is opened.

**Event data:**
```lua
{
    is_open = true
}
```

#### overview:close
Emitted when the overview mode is closed.

**Event data:**
```lua
{
    is_open = false
}
```

## Examples

### Example 1: Window Switcher

Track all open windows and log them when new ones open:

```lua
niri.apply_config({})

local windows = {}

niri.events:on("window:open", function(data)
    windows[data.id] = data.title
    niri.utils.log("Windows open: " .. table.concat(windows, ", "))
end)

niri.events:on("window:close", function(data)
    windows[data.id] = nil
    niri.utils.log("Windows open: " .. table.concat(windows, ", "))
end)
```

### Example 2: Workspace Monitor

Log workspace switches:

```lua
niri.apply_config({})

niri.events:on("workspace:activate", function(data)
    niri.utils.log("Active workspace: " .. data.name .. " (#" .. data.index .. ")")
end)

niri.events:on("workspace:deactivate", function(data)
    niri.utils.log("Workspace deactivated: " .. data.name)
end)
```

### Example 3: Auto-Float Specific Apps

Listen to window open events and track specific applications:

```lua
niri.apply_config({})

niri.events:on("window:open", function(data)
    local title = data.title or ""
    -- Could use this data later to auto-float certain windows
    -- by tracking their properties
    niri.utils.log("New window: " .. title)
end)
```

### Example 4: Focus Indicator

Track which window has focus:

```lua
niri.apply_config({})

local focused_window = nil

niri.events:on("window:focus", function(data)
    focused_window = data.title
    niri.utils.log(">>> Focused: " .. (data.title or "unnamed"))
end)

niri.events:on("window:blur", function(data)
    if focused_window == data.title then
        niri.utils.log("<<< Blurred: " .. (data.title or "unnamed"))
    end
end)
```

### Example 5: Layout Mode Indicator

Monitor layout mode changes:

```lua
niri.apply_config({})

niri.events:on("layout:mode_changed", function(data)
    niri.utils.log("Layout mode: " .. data.mode)
end)
```

### Example 6: Combined Monitoring Script

A comprehensive example that monitors multiple event types:

```lua
niri.apply_config({})

-- Configuration
local config = {
    show_window_events = true,
    show_workspace_events = true,
    show_monitor_events = true,
    show_layout_events = true,
}

-- Event counter
local stats = {
    windows_opened = 0,
    windows_closed = 0,
    workspace_changes = 0,
    monitor_changes = 0,
}

-- Window events
if config.show_window_events then
    niri.events:on("window:open", function(data)
        stats.windows_opened = stats.windows_opened + 1
        niri.utils.log("[WINDOW] Opened: " .. (data.title or "unnamed"))
    end)
    
    niri.events:on("window:close", function(data)
        stats.windows_closed = stats.windows_closed + 1
        niri.utils.log("[WINDOW] Closed: " .. (data.title or "unnamed"))
    end)
    
    niri.events:on("window:focus", function(data)
        niri.utils.log("[FOCUS] -> " .. (data.title or "unnamed"))
    end)
    
    niri.events:on("window:blur", function(data)
        niri.utils.log("[BLUR] <- " .. (data.title or "unnamed"))
    end)
end

-- Workspace events
if config.show_workspace_events then
    niri.events:on("workspace:activate", function(data)
        stats.workspace_changes = stats.workspace_changes + 1
        niri.utils.log("[WORKSPACE] Activated: " .. data.name .. " (idx: " .. data.index .. ")")
    end)
    
    niri.events:on("workspace:deactivate", function(data)
        niri.utils.log("[WORKSPACE] Deactivated: " .. data.name)
    end)
end

-- Monitor events
if config.show_monitor_events then
    niri.events:on("monitor:connect", function(data)
        stats.monitor_changes = stats.monitor_changes + 1
        niri.utils.log("[MONITOR] Connected: " .. data.name .. " (" .. data.connector .. ")")
    end)
    
    niri.events:on("monitor:disconnect", function(data)
        stats.monitor_changes = stats.monitor_changes + 1
        niri.utils.log("[MONITOR] Disconnected: " .. data.name .. " (" .. data.connector .. ")")
    end)
end

-- Layout events
if config.show_layout_events then
    niri.events:on("layout:mode_changed", function(data)
        niri.utils.log("[LAYOUT] Mode changed: " .. data.mode)
    end)
    
    niri.events:on("layout:window_added", function(data)
        niri.utils.log("[LAYOUT] Window added (id: " .. data.id .. ")")
    end)
    
    niri.events:on("layout:window_removed", function(data)
        niri.utils.log("[LAYOUT] Window removed (id: " .. data.id .. ")")
    end)
end

-- Log summary on startup
niri.utils.log("=== Niri Event System Ready ===")
niri.utils.log("Configuration: " .. (config.show_window_events and "Windows " or "") ..
                            (config.show_workspace_events and "Workspaces " or "") ..
                            (config.show_monitor_events and "Monitors " or "") ..
                            (config.show_layout_events and "Layout" or ""))
```

## Best Practices

### 1. Use Meaningful Event Names
Keep track of what each event represents:

```lua
niri.events:on("window:focus", function(data)
    -- Clear, descriptive action
    update_window_title_display(data.title)
end)
```

### 2. Handle Missing Data
Not all events may have complete data. Always check:

```lua
niri.events:on("window:open", function(data)
    local title = data.title or "unknown"
    niri.utils.log("Opened: " .. title)
end)
```

### 3. Use One-Time Handlers for Initialization
Use `niri.events:once()` for events that should only happen once:

```lua
-- Set up notification on first window
niri.events:once("window:open", function(data)
    niri.utils.log("First window opened, system ready!")
end)
```

### 4. Unregister Handlers When Done
If you register handlers dynamically, clean them up:

```lua
local handler_ids = {}

function enable_monitoring()
    table.insert(handler_ids, niri.events:on("window:focus", handle_focus))
end

function disable_monitoring()
    for _, id in ipairs(handler_ids) do
        niri.events:off("window:focus", id)
    end
    handler_ids = {}
end
```

### 5. Keep Handlers Lightweight
Avoid heavy computations in event handlers as they run synchronously:

```lua
-- Good: Quick, responsive
niri.events:on("window:focus", function(data)
    niri.utils.log("Focus: " .. data.title)
end)

-- Bad: Heavy computation blocks the compositor
niri.events:on("window:focus", function(data)
    for i = 1, 1000000 do
        -- expensive computation
    end
end)
```

### 6. Event Ordering
Events fire in a specific order during workspace switches:
1. `workspace:deactivate` (for the old workspace)
2. `workspace:activate` (for the new workspace)

Layout changes typically follow window operations:
1. `window:open` (when created)
2. `layout:window_added` (when placed in layout)

## Combining Events with Runtime API

You can use events together with the Runtime API to create powerful automations:

```lua
niri.apply_config({})

niri.events:on("window:focus", function(data)
    -- Get current window info from Runtime API
    if niri.runtime then
        local window = niri.runtime.get_focused_window()
        if window then
            niri.utils.log("Focused: " .. window.title .. " (app: " .. window.app_id .. ")")
        end
    end
end)
```

## Troubleshooting

### Events Not Firing
1. Check that the event system is initialized (should be automatic)
2. Verify the event name is correct (case-sensitive)
3. Check the compositor logs for any errors

### Handler Not Called
1. Verify the event type name matches exactly
2. Check if the callback function has any errors
3. Use `niri.utils.log()` to debug:

```lua
niri.events:on("window:focus", function(data)
    niri.utils.log("Handler called with data: " .. (data and "table" or "nil"))
end)
```

### Performance Issues
1. Avoid heavy computation in handlers
2. Consider using `niri.events:once()` instead of `niri.events:on()` when appropriate
3. Profile your handlers with timing:

```lua
niri.events:on("window:focus", function(data)
    local start = os.clock()
    -- your handler code
    local elapsed = os.clock() - start
    if elapsed > 0.01 then
        niri.utils.log("Slow handler: " .. elapsed .. "s")
    end
end)
```

## See Also

- [Configuration Guide](LUA_GUIDE.md) - Full Lua configuration documentation
- [Runtime State API](LUA_RUNTIME_STATE_API.md) - Query compositor state
- [Configuration API](docs/CONFIGURATION.md) - Configure Niri settings
