# Niri Lua Quick Start

**Get started with Niri Lua in 5 minutes**

---

## Step 1: Create Your Config File

Create `~/.config/niri/config.lua`:

```lua
-- Niri Lua configuration
niri.utils.log("Niri Lua config loaded!")
```

Restart Niri to apply.

---

## Step 2: Add Basic Keybinds

```lua
-- Quit niri
niri.config.binds:add({ key = "Mod+Shift+E", action = "quit" })

-- Terminal
niri.config.binds:add({ key = "Mod+Return", action = "spawn", args = { "alacritty" } })

-- Application launcher
niri.config.binds:add({ key = "Mod+D", action = "spawn", args = { "rofi", "-show", "drun" } })

-- Window movement
niri.config.binds:add({ key = "Mod+Left", action = "focus-column-left" })
niri.config.binds:add({ key = "Mod+Right", action = "focus-column-right" })
niri.config.binds:add({ key = "Mod+Up", action = "focus-window-up" })
niri.config.binds:add({ key = "Mod+Down", action = "focus-window-down" })

-- Close window
niri.config.binds:add({ key = "Mod+Q", action = "close-window" })
```

---

## Step 3: Configure Layout and Appearance

```lua
-- Layout settings
niri.config.layout.gaps = 16
niri.config.layout.center_focused_column = "never"

-- Border settings
niri.config.layout.border.off = false
niri.config.layout.border.width = 2
niri.config.layout.border.active.color = "#ff8800"
niri.config.layout.border.inactive.color = "#505050"

-- Focus ring (alternative to border)
niri.config.layout.focus_ring.off = true

-- Input settings
niri.config.input.keyboard.repeat_delay = 300
niri.config.input.keyboard.repeat_rate = 50
niri.config.input.touchpad.tap = true
niri.config.input.touchpad.natural_scroll = true

-- Cursor settings
niri.config.cursor.xcursor_theme = "Adwaita"
niri.config.cursor.xcursor_size = 24
```

---

## Step 4: Add Startup Commands

```lua
-- Spawn applications at startup
niri.config.spawn_at_startup:add({ command = { "waybar" } })
niri.config.spawn_at_startup:add({ command = { "swaybg", "-i", "/path/to/wallpaper.png" } })
niri.config.spawn_at_startup:add({ command = { "dunst" } })
```

---

## Step 5: Add Window Rules

```lua
-- Float specific applications
niri.config.window_rules:add({
    matches = { { app_id = "pavucontrol" } },
    open_floating = true,
})

-- Set default size for browsers
niri.config.window_rules:add({
    matches = { { app_id = "firefox" } },
    default_column_width = { proportion = 0.6 },
})

-- Picture-in-picture always floating
niri.config.window_rules:add({
    matches = {
        { app_id = "firefox", title = "Picture-in-Picture" },
    },
    open_floating = true,
})
```

---

## Step 6: Define Workspaces

```lua
-- Named workspaces
niri.config.workspaces:add({ name = "main" })
niri.config.workspaces:add({ name = "web" })
niri.config.workspaces:add({ name = "dev", open_on_output = "eDP-1" })
```

---

## Complete Starter Config

Save this as `~/.config/niri/config.lua`:

```lua
-- ============================================================================
-- Niri Lua Configuration
-- Place at: ~/.config/niri/config.lua
-- ============================================================================

niri.utils.log("Loading Niri configuration...")

-- ============================================================================
-- INPUT SETTINGS
-- ============================================================================

niri.config.input.keyboard.repeat_delay = 300
niri.config.input.keyboard.repeat_rate = 50

niri.config.input.touchpad.tap = true
niri.config.input.touchpad.natural_scroll = true
niri.config.input.touchpad.dwt = true

niri.config.input.mouse.natural_scroll = false

-- ============================================================================
-- LAYOUT SETTINGS
-- ============================================================================

niri.config.layout.gaps = 16
niri.config.layout.center_focused_column = "never"

-- Preset column widths (cycle with Mod+R)
niri.config.layout.preset_column_widths = {
    { proportion = 0.33 },
    { proportion = 0.5 },
    { proportion = 0.67 },
}

-- Default column width
niri.config.layout.default_column_width = { proportion = 0.5 }

-- Border
niri.config.layout.border.off = false
niri.config.layout.border.width = 2
niri.config.layout.border.active.color = "#ff8800"
niri.config.layout.border.inactive.color = "#505050"

-- Focus ring (disabled when using border)
niri.config.layout.focus_ring.off = true

-- ============================================================================
-- CURSOR SETTINGS
-- ============================================================================

niri.config.cursor.xcursor_theme = "Adwaita"
niri.config.cursor.xcursor_size = 24
niri.config.cursor.hide_when_typing = true

-- ============================================================================
-- MISCELLANEOUS
-- ============================================================================

niri.config.prefer_no_csd = true
niri.config.hotkey_overlay.skip_at_startup = true

-- ============================================================================
-- ANIMATIONS
-- ============================================================================

niri.config.animations.slowdown = 1.0

-- ============================================================================
-- WORKSPACES
-- ============================================================================

niri.config.workspaces:add({ name = "main" })
niri.config.workspaces:add({ name = "web" })
niri.config.workspaces:add({ name = "dev" })

-- ============================================================================
-- STARTUP COMMANDS
-- ============================================================================

niri.config.spawn_at_startup:add({ command = { "waybar" } })

-- ============================================================================
-- KEYBINDINGS
-- ============================================================================

-- Session
niri.config.binds:add({ key = "Mod+Shift+E", action = "quit" })
niri.config.binds:add({ key = "Mod+Shift+P", action = "power-off-monitors" })

-- Applications
niri.config.binds:add({ key = "Mod+Return", action = "spawn", args = { "alacritty" } })
niri.config.binds:add({ key = "Mod+D", action = "spawn", args = { "rofi", "-show", "drun" } })

-- Window focus
niri.config.binds:add({ key = "Mod+Left", action = "focus-column-left" })
niri.config.binds:add({ key = "Mod+Right", action = "focus-column-right" })
niri.config.binds:add({ key = "Mod+Up", action = "focus-window-up" })
niri.config.binds:add({ key = "Mod+Down", action = "focus-window-down" })
niri.config.binds:add({ key = "Mod+H", action = "focus-column-left" })
niri.config.binds:add({ key = "Mod+L", action = "focus-column-right" })
niri.config.binds:add({ key = "Mod+J", action = "focus-window-down" })
niri.config.binds:add({ key = "Mod+K", action = "focus-window-up" })

-- Window movement
niri.config.binds:add({ key = "Mod+Shift+Left", action = "move-column-left" })
niri.config.binds:add({ key = "Mod+Shift+Right", action = "move-column-right" })
niri.config.binds:add({ key = "Mod+Shift+Up", action = "move-window-up" })
niri.config.binds:add({ key = "Mod+Shift+Down", action = "move-window-down" })
niri.config.binds:add({ key = "Mod+Shift+H", action = "move-column-left" })
niri.config.binds:add({ key = "Mod+Shift+L", action = "move-column-right" })
niri.config.binds:add({ key = "Mod+Shift+J", action = "move-window-down" })
niri.config.binds:add({ key = "Mod+Shift+K", action = "move-window-up" })

-- Window management
niri.config.binds:add({ key = "Mod+Q", action = "close-window" })
niri.config.binds:add({ key = "Mod+F", action = "maximize-column" })
niri.config.binds:add({ key = "Mod+Shift+F", action = "fullscreen-window" })
niri.config.binds:add({ key = "Mod+V", action = "toggle-window-floating" })

-- Column width
niri.config.binds:add({ key = "Mod+R", action = "switch-preset-column-width" })
niri.config.binds:add({ key = "Mod+Minus", action = "set-column-width", args = { "-10%" } })
niri.config.binds:add({ key = "Mod+Equal", action = "set-column-width", args = { "+10%" } })

-- Workspaces (1-9)
for i = 1, 9 do
    niri.config.binds:add({ key = "Mod+" .. i, action = "focus-workspace", args = { i } })
    niri.config.binds:add({ key = "Mod+Shift+" .. i, action = "move-window-to-workspace", args = { i } })
end

-- Workspace navigation
niri.config.binds:add({ key = "Mod+Page_Up", action = "focus-workspace-up" })
niri.config.binds:add({ key = "Mod+Page_Down", action = "focus-workspace-down" })
niri.config.binds:add({ key = "Mod+U", action = "focus-workspace-up" })
niri.config.binds:add({ key = "Mod+I", action = "focus-workspace-down" })

-- Screenshots
niri.config.binds:add({ key = "Print", action = "screenshot" })
niri.config.binds:add({ key = "Mod+Print", action = "screenshot-window" })

-- ============================================================================
-- WINDOW RULES
-- ============================================================================

-- Float dialogs and utilities
niri.config.window_rules:add({
    matches = { { app_id = "pavucontrol" } },
    open_floating = true,
})

niri.utils.log("Configuration loaded successfully!")
```

---

## KDL vs Lua Differences

The Lua configuration API has some syntax differences from the KDL config:

| KDL Syntax | Lua Syntax |
|------------|------------|
| `Mod+Key` in binds | `"Mod+Key"` (quoted string) |
| `spawn "alacritty"` | `action = "spawn", args = { "alacritty" }` |
| `focus-workspace 1` | `action = "focus-workspace", args = { 1 }` |
| `window-rule { match app-id="..." }` | `matches = { { app_id = "..." } }` |
| `set-column-width "+10%"` | `action = "set-column-width", args = { "+10%" }` |
| Hyphens in names | Underscores in Lua field names |

**Note:** Action names use hyphens (e.g., `focus-column-left`), but Lua table keys use underscores (e.g., `app_id`, `open_floating`).

---

## Verify It Works

Check logs:

```bash
journalctl -eu niri -n 20
```

You should see:

```
Loading Niri configuration...
Configuration loaded successfully!
Applied N reactive config changes
```

---

## Troubleshooting

### Config won't load?

Check logs for errors:

```bash
journalctl -eu niri -n 50
```

### Changes not applying?

Restart Niri:

```bash
niri msg action quit
# Then restart your session
```

### Need help?

- Check `LUA_GUIDE.md` for detailed docs
- Review examples in `examples/`
- Check Niri logs: `journalctl -eu niri`

---

**Happy configuring!**
