# Niri Lua Configuration Guide

**Complete Reference for Lua Configuration in Niri**

This guide covers the Lua configuration API for Niri, including all available settings, keybindings, window rules, and the differences between KDL and Lua configuration formats.

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
12. [KDL vs Lua Differences](#kdl-vs-lua-differences)
13. [Runtime APIs](#runtime-apis)
14. [Troubleshooting](#troubleshooting)

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

-- Log that we're loading
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

The `niri` global table is automatically available in all Lua config files. It provides:

- `niri.config` - Configuration proxy for setting values
- `niri.utils` - Utility functions (logging, etc.)
- `niri.action` - Action execution API
- `niri.state` - Runtime state queries (windows, workspaces, etc.)
- `niri.events` - Event handling system

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

Niri uses a **reactive configuration proxy** that captures your settings and applies them when the config is loaded.

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

---

## Layout Configuration

```lua
-- Gap between windows
niri.config.layout.gaps = 16

-- Center focused column behavior
-- "never" | "always" | "on-overflow"
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

-- XKB settings (set via environment or xkb options)
-- niri.config.input.keyboard.xkb.layout = "us"
-- niri.config.input.keyboard.xkb.options = "ctrl:nocaps"
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
niri.config.input.warp_mouse_to_focus.mode = "center-xy"  -- "center-xy" | "center-xy-always"

-- Workspace auto back-and-forth
niri.config.input.workspace_auto_back_and_forth = true
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

Window rules let you customize behavior for specific applications.

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

## KDL vs Lua Differences

When migrating from KDL to Lua configuration, note these differences:

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

## Runtime APIs

These APIs are available at runtime (after niri starts) for querying state and executing actions.

### State Queries

```lua
-- Query windows (returns empty array during config load)
local windows = niri.state.windows()
for _, win in ipairs(windows) do
    niri.utils.log("Window: " .. win.title)
end

-- Query workspaces
local workspaces = niri.state.workspaces()

-- Query outputs
local outputs = niri.state.outputs()
```

### Action Execution

```lua
-- Execute actions via IPC
niri.action.spawn({ "alacritty" })
niri.action.close_window()
niri.action.focus_workspace(1)
```

### Event Handling

> **Note:** Event system infrastructure is complete, but not all events are wired to compositor code yet.
> See `docs/LUA_CONFIG_STATUS.md` for the current event wiring status.

```lua
-- Listen for events (use colon delimiter for event names)
niri.events:on("window:open", function(event)
    niri.utils.log("Window opened: " .. (event.title or "unknown"))
end)

niri.events:on("workspace:activate", function(event)
    niri.utils.log("Workspace activated: " .. event.name)
end)

-- Currently wired events:
-- - startup
-- - shutdown  
-- - window:open (partial - placeholder data)
-- - workspace:activate
```

### Utility Functions

```lua
-- Logging
niri.utils.log("Info message")
niri.utils.debug("Debug message")
niri.utils.warn("Warning message")
niri.utils.error("Error message")
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

State queries like `niri.state.windows()` return empty arrays during config loading. Use event handlers to access state after niri is running:

```lua
niri.events:on("niri-ready", function()
    local windows = niri.state.windows()
    niri.utils.log("Windows: " .. #windows)
end)
```

---

## Examples

See the `examples/` directory for complete configuration examples:

- `examples/niriv2.lua` - Full configuration example
- `examples/config_api_demo.lua` - API demonstration
- `examples/event_system_demo.lua` - Event handling examples

---

**Document Version:** 2.0 (Reactive API)  
**Last Updated:** December 2025
