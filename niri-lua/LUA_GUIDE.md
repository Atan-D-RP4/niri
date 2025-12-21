# Niri Lua Configuration Guide

Complete reference for Lua configuration in Niri. For a quick introduction, see [LUA_QUICKSTART.md](LUA_QUICKSTART.md). For the complete API specification, see [LUA_SPECIFICATION.md](LUA_SPECIFICATION.md).

---

## Table of Contents

1. [Getting Started](#getting-started)
2. [Configuration Basics](#configuration-basics)
3. [The Reactive Config API](#the-reactive-config-api)
4. [Keybindings](#keybindings)
5. [Layout Configuration](#layout-configuration)
6. [Input Configuration](#input-configuration)
7. [Appearance](#appearance)
8. [Window Rules](#window-rules)
9. [Workspaces](#workspaces)
10. [Startup Commands](#startup-commands)
11. [Animations](#animations)
12. [Runtime APIs](#runtime-apis)
    - [State Queries](#state-queries)
    - [Action Execution](#action-execution)
    - [Utility Functions](#utility-functions)
13. [Event Handling](#event-handling)
14. [Timers and Scheduling](#timers-and-scheduling)
15. [KDL vs Lua Migration](#kdl-vs-lua-migration)
16. [Troubleshooting](#troubleshooting)

---

## Getting Started

### Configuration File Location

Niri looks for Lua configuration in:

1. `~/.config/niri/config.lua` (highest priority)
2. Path specified with `-c` command line option

If no Lua config is found, Niri falls back to KDL configuration.

### Minimal Configuration

```lua
-- ~/.config/niri/config.lua
niri.utils.log("Loading Niri Lua configuration...")

-- Basic keybindings
niri.config.binds:add({ key = "Mod+Return", action = "spawn", args = { "alacritty" } })
niri.config.binds:add({ key = "Mod+Q", action = "close-window" })
niri.config.binds:add({ key = "Mod+Shift+E", action = "quit" })

niri.utils.log("Configuration loaded!")
```

### Testing Your Configuration

```bash
# Run niri with a specific config file
niri -c ~/.config/niri/config.lua

# Check logs for errors
journalctl -eu niri -n 50
```

---

## Configuration Basics

### The `niri` Global

The `niri` global table is automatically available in all Lua config files:

| Namespace | Purpose |
|-----------|---------|
| `niri.config` | Configuration proxy for setting values |
| `niri.utils` | Utility functions (logging, etc.) |
| `niri.action` | Action execution API |
| `niri.state` | Runtime state queries (windows, workspaces, etc.) |
| `niri.events` | Event handling system |
| `niri.loop` | Timer API |

### Lua Syntax Quick Reference

```lua
-- Comments
-- This is a comment

-- Variables
local my_gap = 16
local terminal = "alacritty"

-- Tables (dictionaries)
local bind = {
    key = "Mod+Return",
    action = "spawn",
    args = { "alacritty" },
}

-- Arrays
local commands = { "waybar", "dunst", "swaybg" }

-- String concatenation
local message = "Gap size: " .. tostring(my_gap)

-- Conditionals
if my_gap > 10 then
    niri.utils.log("Large gaps!")
end

-- Loops
for i = 1, 9 do
    niri.config.binds:add({
        key = "Mod+" .. i,
        action = "focus-workspace",
        args = { i }
    })
end
```

---

## The Reactive Config API

Niri uses a **reactive configuration proxy** that captures settings and applies them when config is loaded.

### Setting Scalar Values

```lua
-- Direct field assignment
niri.config.layout.gaps = 16
niri.config.prefer_no_csd = true
niri.config.cursor.xcursor_size = 24

-- Nested fields
niri.config.input.keyboard.repeat_delay = 300
niri.config.layout.border.active.color = "#ff8800"
```

### Bulk Assignment

```lua
-- Set multiple values at once
niri.config.layout = {
    gaps = 16,
    center_focused_column = "never",
    default_column_width = { proportion = 0.5 },
}

-- Nested bulk assignment
niri.config.input.keyboard = {
    repeat_delay = 300,
    repeat_rate = 50,
}
```

### Collection APIs

Collections (binds, window_rules, workspaces, spawn_at_startup) use the `:add()` method:

```lua
-- Add a single item
niri.config.binds:add({
    key = "Mod+Return",
    action = "spawn",
    args = { "alacritty" }
})

-- Add multiple items at once
niri.config.binds:add({
    { key = "Mod+1", action = "focus-workspace", args = { 1 } },
    { key = "Mod+2", action = "focus-workspace", args = { 2 } },
    { key = "Mod+3", action = "focus-workspace", args = { 3 } },
})
```

---

## Keybindings

### Basic Syntax

```lua
niri.config.binds:add({
    key = "MODIFIERS+KEY",
    action = "action-name",
    args = { ... },  -- Optional, depends on action
})
```

### Modifiers

| Modifier | Description |
|----------|-------------|
| `Mod` | Super/Windows key |
| `Ctrl` | Control key |
| `Alt` | Alt key |
| `Shift` | Shift key |
| `Super` | Explicit Super key |

### Common Actions

```lua
-- Spawn applications
niri.config.binds:add({ key = "Mod+Return", action = "spawn", args = { "alacritty" } })
niri.config.binds:add({ key = "Mod+D", action = "spawn", args = { "rofi", "-show", "drun" } })

-- Window management
niri.config.binds:add({ key = "Mod+Q", action = "close-window" })
niri.config.binds:add({ key = "Mod+F", action = "maximize-column" })
niri.config.binds:add({ key = "Mod+Shift+F", action = "fullscreen-window" })
niri.config.binds:add({ key = "Mod+V", action = "toggle-window-floating" })

-- Focus movement
niri.config.binds:add({ key = "Mod+Left", action = "focus-column-left" })
niri.config.binds:add({ key = "Mod+Right", action = "focus-column-right" })
niri.config.binds:add({ key = "Mod+Up", action = "focus-window-up" })
niri.config.binds:add({ key = "Mod+Down", action = "focus-window-down" })

-- Column/window movement
niri.config.binds:add({ key = "Mod+Shift+Left", action = "move-column-left" })
niri.config.binds:add({ key = "Mod+Shift+Right", action = "move-column-right" })

-- Workspaces
niri.config.binds:add({ key = "Mod+1", action = "focus-workspace", args = { 1 } })
niri.config.binds:add({ key = "Mod+Shift+1", action = "move-window-to-workspace", args = { 1 } })

-- Resize
niri.config.binds:add({ key = "Mod+Minus", action = "set-column-width", args = { "-10%" } })
niri.config.binds:add({ key = "Mod+Equal", action = "set-column-width", args = { "+10%" } })

-- Session
niri.config.binds:add({ key = "Mod+Shift+E", action = "quit" })
niri.config.binds:add({ key = "Mod+Shift+P", action = "power-off-monitors" })

-- Screenshots
niri.config.binds:add({ key = "Print", action = "screenshot" })
niri.config.binds:add({ key = "Mod+Print", action = "screenshot-window" })
```

### Generating Keybinds with Loops

```lua
-- Workspace switching (1-9)
for i = 1, 9 do
    niri.config.binds:add({
        key = "Mod+" .. i,
        action = "focus-workspace",
        args = { i }
    })
    niri.config.binds:add({
        key = "Mod+Shift+" .. i,
        action = "move-window-to-workspace",
        args = { i }
    })
end
```

### Bind Options

```lua
-- Allow when screen is locked
niri.config.binds:add({
    key = "XF86AudioRaiseVolume",
    action = "spawn",
    args = { "wpctl", "set-volume", "@DEFAULT_AUDIO_SINK@", "5%+" },
    allow_when_locked = true,
})

-- Cooldown to prevent rapid triggering
niri.config.binds:add({
    key = "Mod+Tab",
    action = "focus-window-down-or-column-right",
    cooldown_ms = 150,
})
```

---

## Layout Configuration

```lua
-- Gap between windows
niri.config.layout.gaps = 16

-- Center focused column behavior: "never" | "always" | "on-overflow"
niri.config.layout.center_focused_column = "never"

-- Preset column widths (cycle with switch-preset-column-width)
niri.config.layout.preset_column_widths = {
    { proportion = 0.33 },
    { proportion = 0.5 },
    { proportion = 0.67 },
}

-- Default column width for new windows
niri.config.layout.default_column_width = { proportion = 0.5 }
-- Or fixed pixel width:
-- niri.config.layout.default_column_width = { fixed = 800 }

-- Struts (reserved screen edges)
niri.config.layout.struts.left = 0
niri.config.layout.struts.right = 0
niri.config.layout.struts.top = 0
niri.config.layout.struts.bottom = 0
```

### Border Configuration

```lua
-- Enable/disable border
niri.config.layout.border.off = false

-- Border width
niri.config.layout.border.width = 2

-- Active window border color
niri.config.layout.border.active.color = "#ff8800"

-- Inactive window border color  
niri.config.layout.border.inactive.color = "#505050"

-- Gradient borders (optional)
niri.config.layout.border.active.gradient = {
    from = "#ff0000",
    to = "#0000ff",
    angle = 45,
}
```

### Focus Ring Configuration

```lua
-- Enable/disable focus ring (alternative to border)
niri.config.layout.focus_ring.off = true

-- Focus ring width
niri.config.layout.focus_ring.width = 4

-- Colors
niri.config.layout.focus_ring.active.color = "#00ff00"
niri.config.layout.focus_ring.inactive.color = "#333333"
```

---

## Input Configuration

### Keyboard

```lua
niri.config.input.keyboard.repeat_delay = 300  -- ms before repeat starts
niri.config.input.keyboard.repeat_rate = 50    -- repeats per second

-- XKB settings
niri.config.input.keyboard.xkb.layout = "us"
niri.config.input.keyboard.xkb.options = "ctrl:nocaps"
```

### Mouse

```lua
niri.config.input.mouse.natural_scroll = false
niri.config.input.mouse.accel_speed = 0.0  -- -1.0 to 1.0
niri.config.input.mouse.accel_profile = "adaptive"  -- "adaptive" | "flat"
```

### Touchpad

```lua
niri.config.input.touchpad.tap = true
niri.config.input.touchpad.natural_scroll = true
niri.config.input.touchpad.dwt = true       -- Disable while typing
niri.config.input.touchpad.dwtp = false     -- Disable while trackpointing
niri.config.input.touchpad.accel_speed = 0.0
niri.config.input.touchpad.accel_profile = "adaptive"
```

### Trackpoint

```lua
niri.config.input.trackpoint.natural_scroll = false
niri.config.input.trackpoint.accel_speed = 0.0
niri.config.input.trackpoint.accel_profile = "flat"
```

### Focus Behavior

```lua
-- Warp mouse to focused window
niri.config.input.warp_mouse_to_focus = true

-- Workspace auto back-and-forth
niri.config.input.workspace_auto_back_and_forth = true

-- Focus follows mouse
niri.config.input.focus_follows_mouse = true
```

---

## Appearance

### Cursor

```lua
niri.config.cursor.xcursor_theme = "Adwaita"
niri.config.cursor.xcursor_size = 24
niri.config.cursor.hide_when_typing = true
niri.config.cursor.hide_after_inactive_ms = 3000
```

### Miscellaneous

```lua
-- Prefer server-side decorations
niri.config.prefer_no_csd = true

-- Skip hotkey overlay at startup
niri.config.hotkey_overlay.skip_at_startup = true

-- Screenshot path
niri.config.screenshot_path = "~/Pictures/Screenshots/screenshot-%Y-%m-%d-%H-%M-%S.png"
```

---

## Window Rules

Window rules customize behavior for specific applications.

### Basic Syntax

```lua
niri.config.window_rules:add({
    matches = { { app_id = "PATTERN" } },
    -- ... rule properties
})
```

### Match Criteria

```lua
-- Match by app_id
matches = { { app_id = "firefox" } }

-- Match by title
matches = { { title = "Settings" } }

-- Match by both (AND)
matches = { { app_id = "firefox", title = "Picture-in-Picture" } }

-- Match multiple patterns (OR)
matches = {
    { app_id = "firefox" },
    { app_id = "chromium" },
}

-- Regex patterns
matches = { { app_id = "^org\\.mozilla\\.firefox$" } }
```

### Rule Properties

```lua
-- Open as floating window
niri.config.window_rules:add({
    matches = { { app_id = "pavucontrol" } },
    open_floating = true,
})

-- Set default column width
niri.config.window_rules:add({
    matches = { { app_id = "firefox" } },
    default_column_width = { proportion = 0.6 },
})

-- Open on specific workspace
niri.config.window_rules:add({
    matches = { { app_id = "slack" } },
    open_on_workspace = "chat",
})

-- Open fullscreen
niri.config.window_rules:add({
    matches = { { app_id = "mpv" } },
    open_fullscreen = true,
})

-- Block out from screencasts
niri.config.window_rules:add({
    matches = { { app_id = "1password" } },
    block_out_from = "screencast",
})
```

---

## Workspaces

```lua
-- Define named workspaces
niri.config.workspaces:add({ name = "main" })
niri.config.workspaces:add({ name = "web" })
niri.config.workspaces:add({ name = "dev" })

-- Workspace on specific output
niri.config.workspaces:add({
    name = "chat",
    open_on_output = "DP-1",
})
```

---

## Startup Commands

```lua
-- Simple command
niri.config.spawn_at_startup:add({ command = { "waybar" } })

-- Command with arguments
niri.config.spawn_at_startup:add({
    command = { "swaybg", "-i", "/path/to/wallpaper.png", "-m", "fill" }
})

-- Multiple startup commands
niri.config.spawn_at_startup:add({ command = { "dunst" } })
niri.config.spawn_at_startup:add({ command = { "nm-applet" } })
niri.config.spawn_at_startup:add({ command = { "blueman-applet" } })
```

---

## Animations

```lua
-- Disable all animations
niri.config.animations.off = true

-- Or slow down animations (for debugging)
niri.config.animations.slowdown = 2.0  -- 2x slower

-- Enable with normal speed
niri.config.animations.off = false
niri.config.animations.slowdown = 1.0
```

---

## Runtime APIs

These APIs are available at runtime for querying state and executing actions.

### State Queries

```lua
-- Query all windows
local windows = niri.state.windows()
for _, win in ipairs(windows) do
    niri.utils.log("Window: " .. win.app_id .. " - " .. win.title)
end

-- Query focused window
local focused = niri.state.focused_window()
if focused then
    niri.utils.log("Focused: " .. focused.title)
end

-- Query workspaces
local workspaces = niri.state.workspaces()
for _, ws in ipairs(workspaces) do
    niri.utils.log("Workspace: " .. (ws.name or ws.idx))
end

-- Query outputs
local outputs = niri.state.outputs()
for _, out in ipairs(outputs) do
    niri.utils.log("Output: " .. out.name .. " " .. out.width .. "x" .. out.height)
end
```

### Action Execution

```lua
-- Execute actions directly
niri.action.spawn("alacritty")
niri.action.close_window()
niri.action.focus_workspace(1)
niri.action.focus_column_left()
niri.action.toggle_window_floating()
```

### Utility Functions

```lua
-- Logging
niri.utils.log("Info message")
niri.utils.debug("Debug message")
niri.utils.warn("Warning message")
niri.utils.error("Error message")

-- Fire-and-forget spawn
niri.utils.spawn({"notify-send", "Hello"})
```

---

## Event Handling

Subscribe to compositor events for dynamic behavior.

### Basic Event Subscription

```lua
-- Subscribe to window open events
niri.events:on("window:open", function(event)
    niri.utils.log("Window opened: " .. event.app_id)
end)

-- Subscribe once (auto-unsubscribes after first call)
niri.events:once("window:focus", function(event)
    niri.utils.log("First focus: " .. event.app_id)
end)

-- Unsubscribe
local id = niri.events:on("window:open", handler)
niri.events:off("window:open", id)
```

### Multi-Event Subscription (vim-style)

You can register the same callback for multiple events at once, similar to Vim's autocmd:

```lua
-- Subscribe to multiple events with one callback
local ids = niri.events:on({"window:open", "window:close", "window:focus"}, function(event)
    niri.utils.log("Window event: " .. event.id)
end)
-- Returns: { ["window:open"] = 1, ["window:close"] = 2, ["window:focus"] = 3 }

-- Subscribe once to multiple events (each fires independently)
niri.events:once({"workspace:create", "output:connect"}, function(event)
    niri.utils.log("First occurrence of either event")
end)

-- Unsubscribe all at once using the returned table
niri.events:off(ids)  -- Removes all handlers
```

### Available Events

| Event | Payload |
|-------|---------|
| `window:open` | `{id, app_id, title, workspace_id}` |
| `window:close` | `{id, app_id, title}` |
| `window:focus` | `{id, app_id, title}` |
| `window:blur` | `{id, title}` |
| `window:title_changed` | `{id, title}` |
| `workspace:create` | `{id, idx, name, output}` |
| `workspace:destroy` | `{id, name, output}` |
| `workspace:activate` | `{id, idx, name, output}` |
| `workspace:deactivate` | `{id, idx, name, output}` |
| `monitor:connect` | `{name, make, model}` |
| `monitor:disconnect` | `{name}` |
| `config:reload` | `{}` |
| `overview:open` | `{}` |
| `overview:close` | `{}` |
| `layout:mode_changed` | `{is_floating}` |

### Event Handler Examples

```lua
-- Auto-move windows by app
niri.events:on("window:open", function(event)
    if event.app_id == "slack" then
        niri.action.move_window_to_workspace("chat")
    elseif event.app_id == "spotify" then
        niri.action.move_window_to_workspace("media")
    end
end)

-- Log workspace changes
niri.events:on("workspace:activate", function(event)
    local name = event.name or ("Workspace " .. event.idx)
    niri.utils.log("Switched to: " .. name)
end)
```

---

## Timers and Scheduling

### Creating Timers

```lua
-- One-shot timer (fires once after delay)
local timer = niri.loop.new_timer()
timer:start(1000, 0, function()
    niri.utils.log("Fired after 1 second")
    timer:close()
end)

-- Repeating timer (fires every interval)
local repeating = niri.loop.new_timer()
repeating:start(0, 500, function()
    niri.utils.log("Tick every 500ms")
end)
-- Later: repeating:close()
```

### Timer Methods

```lua
timer:start(delay_ms, repeat_ms, callback)  -- Start timer
timer:stop()                                 -- Stop timer
timer:again()                                -- Restart with same settings
timer:close()                                -- Clean up resources
timer:is_active()                            -- Check if active
```

### Deferred Execution

```lua
-- Schedule callback for next event loop iteration
niri.schedule(function()
    niri.utils.log("Deferred execution")
end)

-- Use case: break up long operations
niri.events:on("window:open", function(event)
    local window_id = event.id
    
    -- Defer heavy work
    niri.schedule(function()
        do_expensive_analysis(window_id)
    end)
end)
```

### Debouncing Example

```lua
-- Debounce rapid events
local debounce_timer = niri.loop.new_timer()
local pending = nil

niri.events:on("window:open", function(event)
    pending = event
    debounce_timer:stop()
    debounce_timer:start(100, 0, function()
        if pending then
            handle_window(pending)
            pending = nil
        end
    end)
end)
```

---

## KDL vs Lua Migration

### Syntax Mapping

| KDL | Lua |
|-----|-----|
| `spawn "alacritty"` | `action = "spawn", args = { "alacritty" }` |
| `focus-workspace 1` | `action = "focus-workspace", args = { 1 }` |
| `set-column-width "+10%"` | `action = "set-column-width", args = { "+10%" }` |
| `app-id="firefox"` | `app_id = "firefox"` |
| `open-floating` | `open_floating = true` |

### Key Differences

1. **Hyphens vs Underscores**
   - Action names use hyphens: `focus-column-left`, `close-window`
   - Lua table keys use underscores: `app_id`, `open_floating`, `repeat_delay`

2. **Window Rule Matches**
   - KDL: `match { app-id="firefox" }`
   - Lua: `matches = { { app_id = "firefox" } }` (array of match objects)

3. **Arguments**
   - KDL: `spawn "alacritty" "-e" "bash"`
   - Lua: `args = { "alacritty", "-e", "bash" }`

4. **Booleans**
   - KDL: `tap` (presence = true)
   - Lua: `tap = true` (explicit boolean)

5. **Colors**
   - Both: `"#rrggbb"` hex format
   - Lua also supports: `{ r = 255, g = 128, b = 0, a = 255 }`

### Equivalent Configurations

**KDL:**
```kdl
input {
    keyboard {
        repeat-delay 300
        repeat-rate 50
    }
    touchpad {
        tap
        natural-scroll
    }
}

binds {
    Mod+Return { spawn "alacritty"; }
    Mod+Q { close-window; }
    Mod+1 { focus-workspace 1; }
}

window-rule {
    match app-id="firefox"
    default-column-width { proportion 0.6; }
}
```

**Lua:**
```lua
niri.config.input.keyboard.repeat_delay = 300
niri.config.input.keyboard.repeat_rate = 50
niri.config.input.touchpad.tap = true
niri.config.input.touchpad.natural_scroll = true

niri.config.binds:add({ key = "Mod+Return", action = "spawn", args = { "alacritty" } })
niri.config.binds:add({ key = "Mod+Q", action = "close-window" })
niri.config.binds:add({ key = "Mod+1", action = "focus-workspace", args = { 1 } })

niri.config.window_rules:add({
    matches = { { app_id = "firefox" } },
    default_column_width = { proportion = 0.6 },
})
```

---

## Troubleshooting

### Configuration Won't Load

```bash
# Check niri logs
journalctl -eu niri -n 50

# Look for Lua errors in the output
```

### Keybindings Not Working

1. Check that the key name is correct (e.g., `Return` not `Enter`)
2. Check that modifiers are correct (`Mod` = Super key)
3. Check logs for binding parse errors

### Window Rules Not Matching

1. Use `niri msg windows` to see actual `app_id` values
2. Check regex patterns are properly escaped
3. Remember: `matches` is an array of match objects

### State Queries Return Empty

State queries return empty arrays during config loading. Use event handlers for runtime access:

```lua
niri.events:on("startup", function()
    local windows = niri.state.windows()
    niri.utils.log("Windows: " .. #windows)
end)
```

### Script Timeouts

Lua scripts have a 1-second default timeout. If your code times out:

```lua
-- BAD: This will timeout
for i = 1, 1e12 do end

-- GOOD: Use scheduled chunks
local function process_chunk()
    -- Process some items
    if more_work then
        niri.schedule(process_chunk)
    end
end
process_chunk()
```

---

## Examples

See the `examples/` directory in the repository:

- `niriv2.lua` - Full configuration example
- `config_api_demo.lua` - API demonstration
- `event_system_demo.lua` - Event handling examples
- `runtime_state_query.lua` - State query examples

---

## Further Reading

- [LUA_QUICKSTART.md](LUA_QUICKSTART.md) - 5-minute introduction
- [LUA_SPECIFICATION.md](LUA_SPECIFICATION.md) - Complete API specification
- [AGENTS.md](AGENTS.md) - Architecture overview for developers
