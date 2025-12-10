-- Test configuration with keybindings (v2 API)
-- This file tests the niri.config proxy API for keybindings

niri.utils.log("Loading test configuration")

-- Configure keybindings using v2 proxy API
niri.config.binds = {
    -- Spawn terminal with Super+Return
    {
        key = "Super+Return",
        action = "spawn",
        args = { "alacritty" },
    },
    -- Close window with Super+Q
    {
        key = "Super+Q",
        action = "close-window",
    },
    -- Focus window down with Super+J
    {
        key = "Super+J",
        action = "focus-window-down",
    },
    -- Focus window up with Super+K
    {
        key = "Super+K",
        action = "focus-window-up",
    },
    -- Focus column left with Super+H
    {
        key = "Super+H",
        action = "focus-column-left",
    },
    -- Focus column right with Super+L
    {
        key = "Super+L",
        action = "focus-column-right",
    },
    -- Move window down with Super+Shift+J
    {
        key = "Super+Shift+J",
        action = "move-window-down",
    },
    -- Move window up with Super+Shift+K
    {
        key = "Super+Shift+K",
        action = "move-window-up",
    },
    -- Screenshot with Super+Print
    {
        key = "Super+Print",
        action = "screenshot",
    },
    -- Quit with Super+Alt+Q
    {
        key = "Super+Alt+Q",
        action = "quit",
    },
    -- Toggle overview with Super+O
    {
        key = "Super+O",
        action = "toggle-overview",
    },
    -- Show hotkey overlay with Super+Shift+Slash
    {
        key = "Super+Shift+Slash",
        action = "show-hotkey-overlay",
    },
    -- Switch preset column width with Mod+R
    {
        key = "Mod+R",
        action = "switch-preset-column-width",
    },
    -- Consume window left with Mod+BracketLeft
    {
        key = "Mod+BracketLeft",
        action = "consume-or-expel-window-left",
    },
    -- Consume window right with Mod+BracketRight
    {
        key = "Mod+BracketRight",
        action = "consume-or-expel-window-right",
    },
    -- Switch focus between floating and tiling with Mod+Shift+V
    {
        key = "Mod+Shift+V",
        action = "switch-focus-between-floating-and-tiling",
    },
}

-- Basic configuration
niri.config.prefer_no_csd = false

-- Apply the configuration
niri.config:apply()

niri.utils.log("Configuration loaded successfully with " .. #niri.config.binds .. " keybindings")
