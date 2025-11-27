-- ============================================================================
-- Niri Lua Configuration
-- ============================================================================
-- 
-- NOTE: The canonical example configuration has been moved to:
--       examples/niri.lua
--
-- This file is kept for backward compatibility during development.
-- Please refer to examples/niri.lua for the comprehensive, up-to-date
-- example configuration with full documentation.
--
-- ============================================================================

-- Simple minimal config for quick testing
niri.apply_config({
    binds = {
        { key = "Mod+T", action = "spawn", args = { "alacritty" } },
        { key = "Mod+Q", action = "close-window" },
    },
    spawn_at_startup = {
        "waybar",
    },
})

niri.log("Minimal test configuration loaded")
