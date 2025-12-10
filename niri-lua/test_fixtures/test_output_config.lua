-- Test Output Configuration for Niri (v2 API)
-- This file tests all output (monitor) configuration options

niri.utils.log("Loading output configuration test")

-- Configure outputs using v2 proxy API
-- Note: outputs is a named collection, accessed by output name
niri.config.outputs = {
    -- Primary monitor with full configuration
    ["eDP-1"] = {
        scale = 1.5,
        transform = "normal",
        position = { x = 0, y = 0 },
        mode = "1920x1080@60",
        variable_refresh_rate = true,
    },

    -- External monitor
    ["HDMI-A-1"] = {
        scale = 2.0,
        position = { x = 1920, y = 0 },
        mode = "3840x2160@60",
        variable_refresh_rate = true,
    },

    -- Portrait monitor with rotation
    ["DP-1"] = {
        transform = "90",
        position = { x = 3840, y = 0 },
        mode = "2560x1440@144",
    },

    -- Disabled output
    ["DP-2"] = {
        off = true,
    },

    -- Output with flipped transform
    ["HDMI-A-2"] = {
        transform = "flipped-180",
        position = { x = 0, y = 1080 },
    },
}

-- Apply the configuration
niri.config:apply()

niri.utils.log("Output configuration loaded successfully")
