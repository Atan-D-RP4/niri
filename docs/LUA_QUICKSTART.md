# Niri Lua Quick Start

**Get started with Niri Lua in 5 minutes**

---

## Step 1: Create Your Config File

Create `~/.config/niri/config.lua`:

```lua
local niri = require "niri"

niri.log("Niri Lua config loaded!")
```

Restart Niri to apply.

---

## Step 2: Add Basic Keybinds

```lua
local niri = require "niri"

-- Quit
niri.config.set_keybind("Super+Q", "quit")

-- Terminal
niri.config.set_keybind("Super+Return", "spawn alacritty")

-- Application launcher
niri.config.set_keybind("Super+D", "spawn rofi -show drun")

-- Window movement
niri.config.set_keybind("Super+Left", "focus-column-left")
niri.config.set_keybind("Super+Right", "focus-column-right")
niri.config.set_keybind("Super+Up", "focus-up")
niri.config.set_keybind("Super+Down", "focus-down")
```

---

## Step 3: Listen to Window Events

```lua
local niri = require "niri"

-- Log when windows open
niri.events.on("window:open", function(event)
  niri.log("New window: " .. event.window.title)
end)

-- Log active window changes
niri.events.on("window:focus", function(event)
  niri.log("Focus: " .. event.window.title)
end)
```

---

## Step 4: Query Running Windows

```lua
local niri = require "niri"

-- On a hotkey, print all windows
niri.config.set_keybind("Super+W", "show-windows")

-- Create a global to store handler for debugging
local window_list_handler = function(event)
  local windows = niri.state.windows()
  niri.log("=== Open Windows ===")
  for i, win in ipairs(windows) do
    niri.log(string.format(
      "%d. %s (%s)",
      i,
      win.title,
      win.app_id
    ))
  end
end

-- Trigger on workspace change
niri.events.on("workspace:enter", window_list_handler)
```

---

## Step 5: Customize Your Desktop

```lua
local niri = require "niri"

-- Dark theme
niri.config.set_appearance({
  border = {
    width = 4,
    active_color = "#00ff00",
    inactive_color = "#222222",
  },
})

-- Keyboard layout
niri.config.set_input({
  keyboard = {
    xkb_layout = "us",
    repeat_delay = 300,
    repeat_rate = 50,
  },
})

-- Layout
niri.config.set_layout({
  preset = "vertical",
  gaps = 8,
})

-- Animations
niri.config.set_animations({
  window_open = {
    curve = "ease_out_cubic",
    duration_ms = 200,
  },
})
```

---

## Complete Starter Config

Save this as `~/.config/niri/config.lua`:

```lua
#!/usr/bin/env lua
-- Niri Configuration
-- Place at: ~/.config/niri/config.lua

local niri = require "niri"

-- ========== Logging ==========
niri.log("Loading Niri configuration...")

-- ========== Keybinds ==========

-- Session
niri.config.set_keybind("Super+Q", "quit")
niri.config.set_keybind("Super+Escape", "power-off-dialog")

-- Spawn applications
niri.config.set_keybind("Super+Return", "spawn alacritty")
niri.config.set_keybind("Super+D", "spawn rofi -show drun")
niri.config.set_keybind("Super+F", "spawn firefox")

-- Focus
niri.config.set_keybind("Super+Left", "focus-column-left")
niri.config.set_keybind("Super+Right", "focus-column-right")
niri.config.set_keybind("Super+Up", "focus-up")
niri.config.set_keybind("Super+Down", "focus-down")
niri.config.set_keybind("Super+Home", "focus-first")
niri.config.set_keybind("Super+End", "focus-last")

-- Move
niri.config.set_keybind("Super+Shift+Left", "move-column-left")
niri.config.set_keybind("Super+Shift+Right", "move-column-right")
niri.config.set_keybind("Super+Shift+Up", "move-up")
niri.config.set_keybind("Super+Shift+Down", "move-down")

-- Windows
niri.config.set_keybind("Super+F11", "toggle-fullscreen")
niri.config.set_keybind("Super+Space", "toggle-floating")
niri.config.set_keybind("Super+W", "close-window")

-- Workspaces
for i = 1, 9 do
  niri.config.set_keybind("Super+" .. i, "activate-workspace:" .. (i - 1))
  niri.config.set_keybind("Super+Shift+" .. i, "move-window-to-workspace:" .. (i - 1))
end

niri.config.set_keybind("Super+Bracketleft", "activate-workspace-previous")
niri.config.set_keybind("Super+Bracketright", "activate-workspace-next")

-- ========== Input ==========
niri.config.set_input({
  keyboard = {
    xkb_layout = "us",
    repeat_delay = 300,
    repeat_rate = 50,
  },
  mouse = {
    accel = { enabled = true, speed = 0.0 },
    natural_scroll = false,
  },
  touchpad = {
    accel = { enabled = true, speed = 1.0 },
    natural_scroll = true,
    tap_to_click = true,
  },
})

-- ========== Layout ==========
niri.config.set_layout({
  preset = "vertical",
  gaps = 8,
})

-- ========== Appearance ==========
niri.config.set_appearance({
  border = {
    width = 4,
    active_color = "#00aa00",
    inactive_color = "#333333",
  },
})

-- ========== Animations ==========
niri.config.set_animations({
  window_open = {
    curve = "ease_out_cubic",
    duration_ms = 200,
  },
  window_close = {
    curve = "ease_out_cubic",
    duration_ms = 150,
  },
  window_movement = {
    curve = "linear",
    duration_ms = 150,
  },
})

-- ========== Events ==========

-- Log window events
niri.events.on("window:open", function(event)
  niri.log("Window opened: " .. event.window.title)
end)

niri.events.on("window:close", function(event)
  niri.log("Window closed: " .. event.window.title)
end)

niri.events.on("window:focus", function(event)
  niri.log("Focus: " .. event.window.title)
end)

-- Log workspace changes
niri.events.on("workspace:enter", function(event)
  niri.log("Workspace: " .. event.workspace.name)
end)

-- Log when Niri is ready
niri.events.on("niri:ready", function()
  niri.log("Niri is ready!")
  
  -- Print initial window count
  local windows = niri.state.windows()
  niri.log("Initial windows: " .. #windows)
end)

niri.log("Configuration loaded successfully!")
```

---

## Verify It Works

Check logs:

```bash
journalctl -eu niri -n 20
```

You should see:

```
Nov 15 14:30:00 host niri[1234]: Loading Niri configuration...
Nov 15 14:30:00 host niri[1234]: Configuration loaded successfully!
Nov 15 14:30:01 host niri[1234]: Niri is ready!
Nov 15 14:30:01 host niri[1234]: Initial windows: 0
```

---

## Common Keybindings Reference

| Key | Action |
|-----|--------|
| `Super` | Windows key |
| `Super+Return` | Open terminal |
| `Super+Q` | Quit Niri |
| `Super+Arrows` | Focus/move windows |
| `Super+1..9` | Switch workspaces |
| `Super+F11` | Toggle fullscreen |
| `Super+Space` | Toggle floating |

---

## Next Steps

1. **Read Full Guide** - See `LUA_GUIDE.md` for comprehensive docs
2. **Create Plugin** - Build a custom plugin in `~/.config/niri/plugins/`
3. **Explore Events** - Listen to all Niri events in your config
4. **Query State** - Access windows, workspaces, monitors

---

## Troubleshooting

### Config won't load?

Check syntax:

```bash
lua -c ~/.config/niri/config.lua
```

Check logs:

```bash
journalctl -eu niri -n 50
```

### Changes not applying?

Restart Niri:

```bash
niri --quit
niri &
```

### Need help?

- Check `LUA_GUIDE.md` for detailed docs
- Review examples in `examples/plugins/`
- Check Niri logs: `journalctl -eu niri`

---

**Happy configuring! 🚀**
