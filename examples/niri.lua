-- Example Niri Lua Configuration
-- Place this file at ~/.config/niri/niri.lua

-- Log startup message
niri.log("Loading Niri Lua configuration...")

-- Configuration table to hold settings
local config = {
    -- Animation settings
    animations = {
        enabled = true,
        duration = 200,
    },
    -- Window settings
    window = {
        border_width = 2,
        corner_radius = 8,
    },
    -- Layout settings
    layout = {
        preset = "dwindle",
    },
}

-- Helper function to log config
local function log_config()
    niri.log("Configuration loaded:")
    niri.log("  - Animations: enabled=" .. tostring(config.animations.enabled))
    niri.log("  - Border width: " .. config.window.border_width)
    niri.log("  - Layout preset: " .. config.layout.preset)
end

-- Custom keymap example using Lua API
-- Note: This demonstrates the intended API structure for keymaps
-- The actual implementation would need to be added to the NiriApi component
local function setup_keymaps()
    niri.log("Setting up custom keymaps...")

    -- Example keymap: Super+Return to spawn terminal
    -- This would call niri.keymap.set once implemented
    niri.log("Example keymap: Super+T -> spawn terminal")
    niri.keymap.set("normal", "Super+T", function()
        niri.spawn("kitty")
    end)

    -- Example keymap: Super+Q to close window
    niri.log("Example keymap: Super+Q -> close window")
    -- niri.keymap.set("normal", "Super+Q", function()
    --     niri.window.close()
    -- end)

    -- Example keymap: Super+Space to toggle overview
    niri.log("Example keymap: Super+Space -> toggle overview")
    -- niri.keymap.set("normal", "Super+Space", function()
    --     niri.overview.toggle()
    -- end)

    -- Example keymap: Super+Shift+S to take screenshot
    niri.log("Example keymap: Super+Shift+S -> screenshot")
    -- niri.keymap.set("normal", "Super+Shift+S", function()
    --     niri.screenshot.full()
    -- end)
end

-- Initialization function
local function init()
    niri.log("Initializing Niri configuration...")
    log_config()
    setup_keymaps()
    niri.log("Niri configuration ready!")
end

-- Initialize on load
init()

-- Return the config for potential programmatic access
return config
