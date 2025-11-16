-- Test configuration with keybindings
niri.log("Loading test configuration")

-- Basic configuration
prefer_no_csd = false
test_value = 42

-- Define keybindings table
binds = {
    -- Spawn terminal with Super+Return
    {
        key = "Super+Return",
        action = "spawn",
        args = { "alacritty" }
    },
    -- Close window with Super+Q
    {
        key = "Super+Q",
        action = "close-window",
        args = {}
    },
    -- Focus window down with Super+J
    {
        key = "Super+J",
        action = "focus-window-down",
        args = {}
    },
    -- Focus window up with Super+K
    {
        key = "Super+K",
        action = "focus-window-up",
        args = {}
    },
    -- Focus column left with Super+H
    {
        key = "Super+H",
        action = "focus-column-left",
        args = {}
    },
    -- Focus column right with Super+L
    {
        key = "Super+L",
        action = "focus-column-right",
        args = {}
    },
    -- Move window down with Super+Shift+J
    {
        key = "Super+Shift+J",
        action = "move-window-down",
        args = {}
    },
    -- Move window up with Super+Shift+K
    {
        key = "Super+Shift+K",
        action = "move-window-up",
        args = {}
    },
    -- Screenshot with Super+PrintScreen
    {
        key = "Super+Print",
        action = "screenshot",
        args = {}
    },
    -- Exit application with Super+Alt+Q
    {
        key = "Super+Alt+Q",
        action = "exit",
        args = {}
    },
    -- Toggle overview with Super+O
    {
        key = "Super+O",
        action = "overview-toggle",
        args = {}
    },
    -- Show hotkey overlay with Super+F1
    {
        key = "Super+F1",
        action = "hotkey-overlay-toggle",
        args = {}
    },
    -- Suspend with Super+Alt+S
    {
        key = "Super+Alt+S",
        action = "suspend",
        args = {}
    },
    -- Switch preset column width with Mod+R
    {
        key = "Mod+R",
        action = "switch-preset-column-width",
        args = {}
    },
    -- Consume window left with Mod+BracketLeft
    {
        key = "Mod+BracketLeft",
        action = "consume-or-expel-window-left",
        args = {}
    },
    -- Consume window right with Mod+BracketRight
    {
        key = "Mod+BracketRight",
        action = "consume-or-expel-window-right",
        args = {}
    },
    -- Switch focus between floating and tiling with Mod+Shift+V
    {
        key = "Mod+Shift+V",
        action = "switch-focus-between-floating-and-tiling",
        args = {}
    },
}

niri.log("Configuration loaded successfully with " .. #binds .. " keybindings")
