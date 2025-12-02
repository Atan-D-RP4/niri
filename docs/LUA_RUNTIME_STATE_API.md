# Lua Runtime State API

The Runtime State API allows Lua scripts to query the current state of the Niri compositor, including information about windows, workspaces, outputs (monitors), and focus state.

## Overview

The Runtime State API is accessed through `niri.state` and provides four main query functions:

- `niri.state.windows()` - Get all windows
- `niri.state.focused_window()` - Get the currently focused window
- `niri.state.workspaces()` - Get all workspaces
- `niri.state.outputs()` - Get all outputs (monitors)

## Basic Usage

### Query All Windows

```lua
local windows = niri.state.windows()
for i, window in ipairs(windows) do
    niri.utils.log("Window: " .. window.id .. " " .. (window.title or "(no title)"))
end
```

### Get Focused Window

```lua
local focused = niri.state.focused_window()
if focused then
    niri.utils.log("Focused: " .. focused.title)
else
    niri.utils.log("No focused window (focus on layer-shell surface)")
end
```

### Query All Workspaces

```lua
local workspaces = niri.state.workspaces()
for i, ws in ipairs(workspaces) do
    niri.utils.log("Workspace: " .. (ws.name or ws.idx) .. " active: " .. tostring(ws.is_active))
end
```

### Query All Outputs

```lua
local outputs = niri.state.outputs()
for i, output in ipairs(outputs) do
    niri.utils.log("Output: " .. output.name)
end
```

## Window Data Structure

Each window object has the following properties:

```lua
{
    id = 1,                              -- Unique window ID
    title = "Window Title",              -- Window title (string or nil)
    app_id = "org.example.app",         -- Application ID (string or nil)
    pid = 1234,                         -- Process ID (number or nil)
    is_focused = true,                  -- Whether window has focus
    is_floating = false,                -- Whether window is floating
    is_urgent = false,                  -- Whether window requests attention
    workspace_id = 1,                   -- Workspace ID (number or nil)
    
    layout = {                          -- Window layout (only for tiled windows)
        pos_in_scrolling_layout = {1, 2},  -- {column, row} (1-based)
        tile_size = {1920.0, 1080.0},      -- Size of the tile
        window_size = {1920, 1080},        -- Size of window's visual geometry
        tile_pos_in_workspace_view = {0.0, 0.0}, -- Tile position in workspace
        window_offset_in_tile = {0.0, 0.0},     -- Offset within tile
    },
    
    focus_timestamp = {                 -- When window was last focused
        secs = 1234567890,
        nanos = 0,
    }
}
```

## Workspace Data Structure

Each workspace object has the following properties:

```lua
{
    id = 1,                    -- Unique workspace ID (persistent)
    idx = 1,                   -- Index on current monitor (1-based)
    name = "workspace-1",      -- Workspace name (string or nil)
    output = "HDMI-1",         -- Output name (string or nil)
    is_active = true,          -- Currently visible on its monitor
    is_focused = true,         -- Has focus (only one across all outputs)
    is_urgent = false,         -- Has urgent window
    active_window_id = 123,    -- ID of active window (number or nil)
}
```

## Output Data Structure

Each output object has the following properties:

```lua
{
    name = "HDMI-1",           -- Output name
    make = "Samsung",          -- Manufacturer
    model = "LU28E590DS",      -- Model name
    serial = "SN123456",       -- Serial number (string or nil)
    
    physical_size = {640, 360}, -- Physical dimensions in mm (table or nil)
    
    modes = {                   -- Available display modes
        {
            width = 3840,
            height = 2160,
            refresh_rate = 60000,  -- in millihertz (60000 = 60 Hz)
            is_preferred = true,
        }
    },
    
    current_mode = 0,           -- Index into modes array (number or nil)
    is_custom_mode = false,     -- Whether current mode is custom
    is_enabled = true,          -- Whether output is active
    
    vrr_supported = true,       -- Variable refresh rate support
    vrr_enabled = false,        -- VRR currently enabled
    
    logical = {                 -- Logical output info (nil if disabled)
        size = {1920, 1080},   -- Logical size in pixels
        pos = {0, 0},          -- Position in logical layout
        scale = 1.0,           -- Display scale factor
    }
}
```

## Practical Examples

### Example 1: Window Status Summary

Count tiled, floating, and urgent windows:

```lua
local windows = niri.state.windows()

local total_floating = 0
local total_tiled = 0
local total_urgent = 0

for _, window in ipairs(windows) do
    if window.is_floating then
        total_floating = total_floating + 1
    else
        total_tiled = total_tiled + 1
    end
    if window.is_urgent then
        total_urgent = total_urgent + 1
    end
end

niri.utils.log(string.format(
    "Tiled: %d, Floating: %d, Urgent: %d",
    total_tiled,
    total_floating,
    total_urgent
))
```

### Example 2: Find Window by Title

Search for windows matching a pattern:

```lua
local function find_windows_by_title(pattern)
    local matches = {}
    for _, window in ipairs(niri.state.windows()) do
        if window.title and window.title:match(pattern) then
            table.insert(matches, window)
        end
    end
    return matches
end

local firefox_windows = find_windows_by_title("Firefox")
for _, win in ipairs(firefox_windows) do
    niri.utils.log("Found Firefox: " .. win.title)
end
```

### Example 3: Count Windows Per Workspace

Analyze workspace layout:

```lua
local function count_windows_per_workspace()
    local windows = niri.state.windows()
    local workspaces = niri.state.workspaces()
    local counts = {}

    for _, ws in ipairs(workspaces) do
        counts[ws.id] = 0
    end

    for _, window in ipairs(windows) do
        if window.workspace_id then
            counts[window.workspace_id] = (counts[window.workspace_id] or 0) + 1
        end
    end

    return counts
end

local counts = count_windows_per_workspace()
for _, ws in ipairs(niri.state.workspaces()) do
    local name = ws.name or ("Workspace " .. ws.idx)
    local count = counts[ws.id] or 0
    niri.utils.log(string.format("%s: %d windows", name, count))
end
```

### Example 4: Monitor Information

Display information about connected monitors:

```lua
local outputs = niri.state.outputs()

for _, output in ipairs(outputs) do
    niri.utils.log(output.name .. " - " .. output.make .. " " .. output.model)
    
    if output.logical then
        local l = output.logical
        niri.utils.log(string.format(
            "  Size: %dx%d, Scale: %.2f, Pos: (%d, %d)",
            l.width, l.height,
            l.scale,
            l.x, l.y
        ))
    end
    
    if output.current_mode then
        local idx = output.current_mode + 1  -- modes array is 0-indexed
        local mode = output.modes[idx]
        if mode then
            niri.utils.log(string.format(
                "  Mode: %dx%d @ %.1f Hz",
                mode.width,
                mode.height,
                mode.refresh_rate / 1000.0
            ))
        end
    end
end
```

### Example 5: Application-Specific Logic

Implement application-aware behavior:

```lua
-- Check if a terminal is focused
local focused = niri.state.focused_window()
if focused and (focused.app_id == "org.gnome.Terminal" or 
               focused.app_id == "kitty") then
    niri.utils.log("Terminal is focused")
end
```

## Integration with Event Hooks

The Runtime State API works perfectly with Niri's Lua event hooks:

```lua
niri.on("window:open", function(event)
    -- Get current state to make decisions
    local all_windows = niri.state.windows()
    
    -- Do something based on current state
    niri.utils.log("Window opened. Total windows now: " .. #all_windows)
end)

niri.on("workspace:activate", function(event)
    local workspaces = niri.state.workspaces()
    for _, ws in ipairs(workspaces) do
        if ws.id == event.workspace.id then
            local windows_count = 0
            for _, win in ipairs(niri.state.windows()) do
                if win.workspace_id == event.workspace.id then
                    windows_count = windows_count + 1
                end
            end
            niri.utils.log(string.format(
                "Activated %s with %d windows",
                ws.name or ws.idx,
                windows_count
            ))
        end
    end
end)
```

## Performance Notes

- `windows()`, `workspaces()`, and `outputs()` query the compositor state and return new tables each time
- The queries are synchronous and block until the compositor responds
- For best performance, cache results if you need to query multiple times in a loop:

```lua
-- Good: cache the results
local windows = niri.state.windows()
for _, win in ipairs(windows) do
    -- ... do something
end
for _, win in ipairs(windows) do
    -- ... do something else with same data
end

-- Avoid: multiple queries
for _, win in ipairs(niri.state.windows()) do
    -- ... 
end
for _, win in ipairs(niri.state.windows()) do  -- Second query
    -- ...
end
```

## Thread Safety

The Runtime State API is safe to call from Lua event hooks and other callbacks. The implementation uses event loop message passing to safely query state on the main compositor thread.

## Consistency Notes

Different parts of state are not guaranteed to be perfectly consistent between consecutive queries. For example:

- A window might be reported as open in `windows()` but then closed before you query it again
- Workspace and window queries are separate, so a window might be reported as belonging to a workspace that was just deleted

For applications requiring strict consistency, query all state at once and process it together:

```lua
-- Get a consistent snapshot of state
local windows = niri.state.windows()
local workspaces = niri.state.workspaces()
local outputs = niri.state.outputs()

-- Now process them together, knowing they came from a single point in time
-- (though there may have been changes between each query)
```
