# Niri Lua Configuration & Plugin Guide

**Complete Reference for Lua in Niri**

This comprehensive guide covers everything you need to know about configuring and extending Niri with Lua scripts and plugins.

---

## Table of Contents

1. [Getting Started](#getting-started)
2. [Configuration Basics](#configuration-basics)
3. [Core Concepts](#core-concepts)
4. [Configuration API](#configuration-api)
5. [State Queries](#state-queries)
6. [Event Handling](#event-handling)
7. [Plugin Development](#plugin-development)
8. [Advanced Topics](#advanced-topics)
9. [Troubleshooting](#troubleshooting)
10. [Examples](#examples)

---

## Getting Started

### Installation

Niri with Lua support is available through:

```bash
# From source with Lua enabled
git clone https://github.com/sodiboo/niri.git
cd niri
cargo build --release --features lua
sudo cargo install --path .

# Or if using your distro's package
# (requires niri-lua package)
```

### Configuration Files

Niri looks for Lua configuration in these locations (in order):

1. **`~/.config/niri/config.lua`** - User configuration (highest priority)
2. **`~/.config/niri/config.kdl`** - KDL configuration (fallback)
3. **`/etc/niri/config.kdl`** - System default (lowest priority)

### First Configuration

Create `~/.config/niri/config.lua`:

```lua
-- ~/.config/niri/config.lua
-- Niri Lua Configuration Example

local niri = require "niri"

-- Log that config is loading
niri.log("Loading Niri configuration...")

-- Basic keybinds
niri.config.set_keybind("Super+Q", "quit")
niri.config.set_keybind("Super+Return", "spawn term")

-- Log on ready
niri.events.on("niri:ready", function()
  niri.log("Niri is ready!")
end)

niri.log("Configuration loaded successfully")
```

Restart Niri to apply changes:

```bash
# Reload configuration (if supported)
niri --quit
niri &

# Or use systemctl if using niri.service
systemctl --user restart niri
```

---

## Configuration Basics

### Lua Syntax Primer

If you're new to Lua, here are the basics:

```lua
-- Comments use --

-- Variables
local x = 10
local name = "niri"
local enabled = true

-- Tables (like dictionaries or arrays)
local config = {
  width = 1920,
  height = 1080,
  theme = "dark",
}

-- Access table values
print(config.width)        -- 1920
print(config["width"])     -- 1920

-- Arrays/lists
local items = { "one", "two", "three" }
print(items[1])            -- "one"

-- Strings
local msg = "Hello, " .. "Niri"  -- Concatenation
local formatted = string.format("Size: %d x %d", 1920, 1080)

-- Functions
local function add(a, b)
  return a + b
end
print(add(5, 3))           -- 8

-- Loops
for i = 1, 3 do
  print(i)                 -- Prints 1, 2, 3
end

-- Conditionals
if x > 5 then
  print("x is large")
elseif x > 0 then
  print("x is positive")
else
  print("x is not positive")
end
```

### Module System

The `require` function loads Lua modules:

```lua
-- Load niri module (always available)
local niri = require "niri"

-- Load your own modules
local helpers = require "helpers"  -- Loads ./helpers.lua

-- Module search paths
-- - ~/.config/niri/?.lua
-- - ~/.config/niri/?/init.lua
-- - ~/.local/share/niri/plugins/?.lua
```

---

## Core Concepts

### 1. Logging

Output messages to Niri's log (visible with `journalctl -eu niri`):

```lua
niri.log("This is a message")           -- info level
niri.debug("Debug information")         -- debug level
niri.info("Informational message")      -- info level
niri.warn("Warning message")            -- warning level
niri.error("Error message")             -- error level

-- Formatted logging
local version = niri.version_string()
niri.log("Niri version: " .. version)

-- Complex objects (use string.format)
local win = { id = 1, title = "Firefox" }
niri.log(string.format("Window: %s (ID: %d)", win.title, win.id))
```

### 2. Version Information

Check Niri version:

```lua
local version = niri.version()
-- { major = 0, minor = 1, patch = 0, is_debug = false }

local version_str = niri.version_string()
-- "Niri 0.1.0"

-- Conditional behavior based on version
if version.major >= 1 then
  niri.log("Running Niri 1.0+")
else
  niri.log("Running Niri 0.x")
end
```

### 3. Process Spawning

Execute external commands:

```lua
-- Non-blocking spawn
niri.spawn("firefox")
niri.spawn("alacritty", {
  cwd = os.getenv("HOME"),
})

-- Blocking spawn (wait for exit)
local exit_code = niri.spawn_blocking("echo 'done'")
niri.log("Command exited with code: " .. exit_code)

-- With environment variables
niri.spawn("myapp", {
  env = {
    MY_VAR = "value",
    ANOTHER = "setting",
  },
})

-- With custom working directory
niri.spawn("command", {
  cwd = "/tmp",
})
```

### 4. Event System

Respond to Niri events:

```lua
-- Basic event listener
niri.events.on("window:open", function(event)
  niri.log("New window: " .. event.window.title)
end)

-- One-time listener
niri.events.once("workspace:enter", function(event)
  niri.log("Entered workspace (will fire only once)")
end)

-- Remove listener
local handler_id = niri.events.on("action", function(event)
  -- This will be called
end)
niri.events.off("action", handler_id)  -- Won't be called anymore
```

### 5. Plugin Lifecycle

Understand how plugins load and run:

```lua
-- 1. Configuration loaded (initialization)
niri.log("Plugin is initializing...")

-- 2. Setup phase
local function setup()
  niri.log("Setting up plugin...")
  -- Register keybinds, listeners, etc.
end

-- 3. Ready event
niri.events.on("niri:ready", function()
  niri.log("Niri is ready, plugin is active")
end)

-- 4. Cleanup before reload
niri.events.on("plugin:unload", function()
  niri.log("Plugin is being unloaded")
  -- Clean up resources
end)
```

---

## Configuration API

### Animations

Control animation timings and curves:

```lua
-- Get current animation config
local anim = niri.config.get_animations()
-- {
--   window_open = { curve = "ease_out_back", duration_ms = 200 },
--   window_close = { curve = "ease_out_back", duration_ms = 200 },
--   ...
-- }

-- Set animation config (partial update)
niri.config.set_animations({
  window_open = {
    curve = "ease_out_cubic",
    duration_ms = 300,
  },
  window_movement = {
    curve = "linear",
    duration_ms = 150,
  },
})

-- Available curves
-- - "linear" - No acceleration
-- - "ease_out_cubic" - Quick start, smooth end
-- - "ease_out_back" - Slightly bouncy
-- - "ease_out_sine" - Smooth deceleration
```

### Input Configuration

Configure keyboard, mouse, and touchpad:

```lua
-- Get input config
local input = niri.config.get_input()

-- Set keyboard layout
niri.config.set_input({
  keyboard = {
    xkb_layout = "us,de",      -- Multiple layouts
    xkb_variant = "dvorak",    -- Or specific variant
    xkb_options = "grp:alt_shift_toggle",
    repeat_delay = 600,        -- ms before repeat
    repeat_rate = 25,          -- repeats per second
  },
})

-- Configure mouse acceleration
niri.config.set_input({
  mouse = {
    accel = { enabled = true, speed = 0.0 },
    natural_scroll = false,
  },
})

-- Configure touchpad
niri.config.set_input({
  touchpad = {
    accel = { enabled = true, speed = 1.0 },
    natural_scroll = true,
    tap_to_click = true,
  },
})
```

### Layout Configuration

Customize tiling layout:

```lua
-- Get current layout
local layout = niri.config.get_layout()
-- { preset = "vertical", gaps = 8, struts = {...} }

-- Change layout preset
niri.config.set_layout({
  preset = "vertical",  -- or "horizontal", "paper"
  gaps = 12,           -- Space between windows
})

-- Configure struts (reserved screen space)
niri.config.set_layout({
  struts = {
    top = 32,      -- Panel height at top
    bottom = 0,
    left = 0,
    right = 0,
  },
})
```

### Appearance Configuration

Customize visuals:

```lua
-- Set border style
niri.config.set_appearance({
  border = {
    width = 4,
    active_color = "#ffaa00",      -- Hex color
    inactive_color = "#333333",
    active_gradient_angle = 45,    -- Optional gradient
  },
})

-- Set background
niri.config.set_appearance({
  background_image = os.getenv("HOME") .. "/Pictures/wallpaper.png",
  background_blur = 10,  -- Blur amount (0-100)
})
```

### Keybindings

Define and manage keyboard shortcuts:

```lua
-- Set a keybind
niri.config.set_keybind("Super+Q", "quit")
niri.config.set_keybind("Super+Return", "spawn alacritty")
niri.config.set_keybind("Super+M", "toggle-fullscreen")

-- Get all keybinds
local binds = niri.config.get_keybinds()
for key, action in pairs(binds) do
  niri.log(string.format("%s => %s", key, action))
end

-- Remove a keybind
niri.config.remove_keybind("Super+Q")

-- Multi-key combos
niri.config.set_keybind("Super+Ctrl+Alt+L", "lock-screen")

-- With modifiers
-- Super (Windows key), Ctrl, Alt, Shift
-- Example: "Super+Shift+N" or "Ctrl+Alt+T"
```

---

## State Queries

### Windows

Query and filter windows:

```lua
-- Get all windows
local windows = niri.state.windows()

-- Iterate and print
for i, win in ipairs(windows) do
  niri.log(string.format(
    "Window %d: %s (app: %s, floating: %s)",
    i,
    win.title,
    win.app_id,
    tostring(win.is_floating)
  ))
end

-- Get specific window
local active = niri.state.active_window()
if active then
  niri.log("Active: " .. active.title)
end

-- Find window by ID
local win = niri.state.window_by_id(42)

-- Find by app ID
local firefox = niri.state.window_by_app_id("firefox")

-- Windows on workspace
local ws_windows = niri.state.windows_on_workspace(1)
niri.log("Workspace 1 has " .. #ws_windows .. " windows")

-- Window properties
local win = niri.state.active_window()
if win then
  print(win.id)           -- Unique ID
  print(win.title)        -- Window title
  print(win.app_id)       -- Application ID
  print(win.is_floating)  -- Floating or tiled
  print(win.workspace_id) -- Workspace ID
  print(win.bounds)       -- { x, y, width, height }
end
```

### Workspaces

Query workspaces:

```lua
-- Get all workspaces
local workspaces = niri.state.workspaces()

-- Iterate
for i, ws in ipairs(workspaces) do
  niri.log(string.format(
    "Workspace: %s (index: %d, windows: %d)",
    ws.name,
    ws.index,
    ws.window_count
  ))
end

-- Active workspace
local active = niri.state.active_workspace()
niri.log("Active: " .. active.name)

-- Get by name or ID
local ws1 = niri.state.workspace_by_name("1")
local ws_named = niri.state.workspace_by_id(42)

-- Workspace properties
local ws = niri.state.active_workspace()
print(ws.id)              -- Unique ID
print(ws.name)            -- Workspace name
print(ws.index)           -- Numeric index
print(ws.monitor_index)   -- Which monitor
print(ws.window_count)    -- Number of windows
print(ws.layout_mode)     -- "tiling" or "floating"
```

### Monitors

Query connected monitors:

```lua
-- Get all monitors
local monitors = niri.state.monitors()

-- Iterate
for i, mon in ipairs(monitors) do
  niri.log(string.format(
    "Monitor %d: %s (%s) @ %.1fx",
    mon.index,
    mon.model,
    mon.make,
    mon.current_scale
  ))
end

-- Active monitor
local active = niri.state.active_monitor()

-- By index
local mon0 = niri.state.monitor_at_index(0)

-- Monitor properties
local mon = niri.state.active_monitor()
print(mon.index)          -- 0, 1, 2...
print(mon.name)           -- e.g., "HDMI-1"
print(mon.make)           -- e.g., "Dell"
print(mon.model)          -- e.g., "U2415"
print(mon.refresh_rate)   -- e.g., 60
print(mon.current_scale)  -- e.g., 1.0
print(mon.layout)         -- Position: { x, y, width, height }
```

### Advanced Filtering

Combine queries to find specific windows:

```lua
-- Find floating windows
local function floating_windows()
  local result = {}
  for _, win in ipairs(niri.state.windows()) do
    if win.is_floating then
      table.insert(result, win)
    end
  end
  return result
end

-- Find windows on specific workspace
local function windows_by_workspace(ws_name)
  local ws = niri.state.workspace_by_name(ws_name)
  if not ws then return {} end
  return niri.state.windows_on_workspace(ws.id)
end

-- Find window by title pattern
local function find_window_by_title(pattern)
  for _, win in ipairs(niri.state.windows()) do
    if string.match(win.title, pattern) then
      return win
    end
  end
end

-- Find windows from specific application
local function app_windows(app_id)
  local result = {}
  for _, win in ipairs(niri.state.windows()) do
    if win.app_id == app_id then
      table.insert(result, win)
    end
  end
  return result
end

-- Usage
niri.log("Firefox windows: " .. #app_windows("firefox"))
niri.log("Floating windows: " .. #floating_windows())
```

---

## Event Handling

### Available Events

```lua
-- Window events
niri.events.on("window:open", function(ev)
  niri.log("Opened: " .. ev.window.title)
end)

niri.events.on("window:close", function(ev)
  niri.log("Closed: " .. ev.window.title)
end)

niri.events.on("window:focus", function(ev)
  if ev.old_focus then
    niri.log("Focus: " .. ev.old_focus.title .. " → " .. ev.window.title)
  end
end)

niri.events.on("window:title_changed", function(ev)
  niri.log("Title changed: " .. ev.window.title)
end)

-- Workspace events
niri.events.on("workspace:enter", function(ev)
  niri.log("Entered workspace: " .. ev.workspace.name)
end)

niri.events.on("workspace:leave", function(ev)
  niri.log("Left workspace: " .. ev.workspace.name)
end)

niri.events.on("workspace:layout_changed", function(ev)
  niri.log("Layout is now: " .. ev.workspace.layout_mode)
end)

-- Monitor events
niri.events.on("monitor:connect", function(ev)
  niri.log("Monitor connected: " .. ev.monitor.model)
end)

niri.events.on("monitor:disconnect", function(ev)
  niri.log("Monitor disconnected: " .. ev.monitor.model)
end)

-- System events
niri.events.on("niri:ready", function()
  niri.log("Niri initialization complete")
end)
```

### Event Patterns

Common patterns for event handling:

```lua
-- Track active window
local active_window = nil

niri.events.on("window:focus", function(ev)
  active_window = ev.window
  niri.log("Active window: " .. active_window.title)
end)

-- Accumulate state
local window_count = 0

niri.events.on("window:open", function(ev)
  window_count = window_count + 1
  niri.log("Window count: " .. window_count)
end)

niri.events.on("window:close", function(ev)
  window_count = window_count - 1
  niri.log("Window count: " .. window_count)
end)

-- Conditional reactions
niri.events.on("window:open", function(ev)
  if string.match(ev.window.app_id, "chrome") then
    niri.log("Browser opened!")
  end
end)

-- Debouncing (ignore rapid events)
local last_focus_time = 0
niri.events.on("window:focus", function(ev)
  local now = os.time() * 1000
  if now - last_focus_time > 100 then  -- 100ms minimum
    niri.log("Focus: " .. ev.window.title)
    last_focus_time = now
  end
end)
```

---

## Plugin Development

### Plugin Structure

A plugin is a Lua file with this structure:

```lua
-- ~/.config/niri/plugins/my-plugin.lua

local niri = require "niri"

-- Plugin metadata
local metadata = {
  name = "my-plugin",
  version = "1.0.0",
  author = "Your Name",
  description = "What it does",
  license = "MIT",
  dependencies = {},  -- Other plugins required
}

-- Initialization
local function setup()
  niri.log("Initializing my-plugin...")
  
  -- Register keybinds
  niri.config.set_keybind("Super+P", "my-plugin.action")
  
  -- Register event listeners
  niri.events.on("window:open", on_window_open)
end

-- Main logic
local function on_window_open(event)
  niri.log("New window: " .. event.window.title)
end

-- Initialization on load
setup()

-- Return metadata (optional but recommended)
return metadata
```

### Plugin Paths

Niri looks for plugins in:

1. **`~/.config/niri/plugins/`** - User plugins (highest priority)
2. **`/usr/local/share/niri/plugins/`** - System plugins
3. **`/usr/share/niri/plugins/`** - Vendor plugins

### Plugin Persistence

Save plugin state to survive restarts:

```lua
local niri = require "niri"

-- Save state
local state = {
  counter = 42,
  theme = "dark",
  last_window_id = nil,
}

-- Serialize (JSON-like table)
function save_state()
  -- In real code, use JSON library or custom serialization
  niri.log("State would be saved here")
end

-- Restore on init
function load_state()
  -- In real code, load from storage
  niri.log("State would be loaded here")
end

-- Save on important events
niri.events.on("niri:shutdown", save_state)
niri.events.on("plugin:unload", save_state)
```

### Publishing Plugins

To share your plugin:

1. Create GitHub repository
2. Name it `niri-<plugin-name>`
3. Add `niri-plugin` topic
4. Create README with:
   - Description
   - Installation instructions
   - Usage examples
   - Screenshots
5. Submit to Niri plugin registry (when available)

---

## Advanced Topics

### Custom Modules

Create reusable utilities:

```lua
-- ~/.config/niri/lib/helpers.lua

local helpers = {}

function helpers.window_by_title(title)
  for _, win in ipairs(niri.state.windows()) do
    if win.title == title then
      return win
    end
  end
  return nil
end

function helpers.workspace_index(ws)
  return ws.index or 0
end

return helpers

-- Usage in main config:
local helpers = require "lib.helpers"
local firefox = helpers.window_by_title("Mozilla Firefox")
```

### Performance Optimization

Tips for efficient Lua code:

```lua
-- Minimize table allocations in hot paths
local windows_cache = {}

niri.events.on("window:open", function(ev)
  -- Bad: creates new table every time
  local windows = niri.state.windows()
  
  -- Good: reuse cached table
  windows_cache = niri.state.windows()
  niri.log("Total windows: " .. #windows_cache)
end)

-- Avoid expensive operations in event handlers
niri.events.on("window:focus", function(ev)
  -- OK for infrequent events, but not ideal for rapid events
  local all_wins = niri.state.windows()  -- O(n) operation
end)

-- Use early returns
local function check_window(win)
  if not win then return false end
  if win.is_floating then return false end
  if win.workspace_id ~= 1 then return false end
  return true
end
```

### Debugging

Enable debug logging:

```lua
-- Set environment variable
-- export NIRI_LUA_DEBUG=1

-- Or check in code
if niri.version().is_debug then
  niri.debug("Debug mode enabled")
end

-- Print complex objects
local function dump(obj, indent)
  indent = indent or 0
  for k, v in pairs(obj) do
    print(string.rep(" ", indent) .. k .. ": " .. tostring(v))
    if type(v) == "table" then
      dump(v, indent + 2)
    end
  end
end

dump(niri.state.active_window())
```

---

## Troubleshooting

### Configuration Won't Load

```bash
# Check logs
journalctl -eu niri -n 50

# Validate Lua syntax
lua -c ~/.config/niri/config.lua

# Test with simple config
echo 'local niri = require "niri"; niri.log("OK")' > ~/.config/niri/config.lua
```

### Performance Issues

```bash
# Profile event handlers
# Use niri.log() strategically to measure timing

local start = os.time() * 1000
-- expensive operation
local elapsed = os.time() * 1000 - start
niri.log(string.format("Operation took %dms", elapsed))
```

### Module Not Found

```lua
-- Ensure full path is correct
-- ~/.config/niri/plugins/mylib.lua
local mylib = require "plugins.mylib"  -- ✓ Correct
local mylib = require "mylib"          -- ✗ Won't find it
```

### State Queries Return Empty

```lua
-- Windows must exist
local wins = niri.state.windows()
if #wins == 0 then
  niri.log("No windows exist yet")
end

-- Queries only work after niri:ready
niri.events.on("niri:ready", function()
  local wins = niri.state.windows()  -- ✓ Works
  niri.log("Initial windows: " .. #wins)
end)
```

---

## Examples

### Example 1: Window Switcher

```lua
-- Quick window switcher on Super+W

local niri = require "niri"

niri.config.set_keybind("Super+W", "window-switcher.show")

-- Show list of windows
niri.events.on("window-switcher:show", function()
  local windows = niri.state.windows()
  
  if #windows == 0 then
    niri.log("No windows open")
    return
  end
  
  for i, win in ipairs(windows) do
    niri.log(string.format(
      "[%d] %s (%s)",
      i,
      win.title:sub(1, 40),  -- First 40 chars
      win.app_id
    ))
  end
end)
```

### Example 2: Workspace Manager

```lua
-- Quick workspace switching

local niri = require "niri"

-- Switch to workspace by number
for i = 1, 9 do
  niri.config.set_keybind(
    "Super+" .. i,
    "workspace-activate:" .. (i - 1)
  )
  
  -- Also create on-demand
  niri.config.set_keybind(
    "Super+Shift+" .. i,
    "workspace-create:" .. tostring(i - 1)
  )
end

-- Previous/next workspace
niri.config.set_keybind("Super+N", "workspace-next")
niri.config.set_keybind("Super+P", "workspace-previous")

-- Show workspace info
niri.events.on("workspace:enter", function(ev)
  local ws = ev.workspace
  niri.log(string.format(
    "Workspace: %s (%d windows)",
    ws.name,
    ws.window_count
  ))
end)
```

### Example 3: Auto-floating for Dialogs

```lua
-- Automatically float dialog windows

local niri = require "niri"

niri.events.on("window:open", function(ev)
  local win = ev.window
  
  -- List of apps that should be floating
  local float_apps = {
    "org.gnome.Calendar",
    "pavucontrol",
    "obs",
    "blender",
  }
  
  for _, app in ipairs(float_apps) do
    if win.app_id == app then
      niri.window.set_floating(win.id, true)
      niri.log("Auto-floated: " .. win.title)
      break
    end
  end
end)
```

---

## Resources

- **Lua Documentation**: https://www.lua.org/manual/5.2/
- **Niri Repository**: https://github.com/sodiboo/niri
- **Example Plugins**: See `examples/plugins/` in Niri repository
- **Type Definitions**: `docs/niri.d.lua` for IDE support

---

**Document Version:** 1.0  
**Last Updated:** November 15, 2025  
**Author:** OpenCode Assistant
