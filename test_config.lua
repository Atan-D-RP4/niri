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
}

niri.log("Configuration loaded successfully with " .. #binds .. " keybindings")
