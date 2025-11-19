-- Test output configuration
-- This file demonstrates all supported output configuration options

outputs = {
    -- Primary monitor with full configuration
    {
        name = "eDP-1",
        scale = 1.5,
        transform = "normal",
        position = { x = 0, y = 0 },
        mode = "1920x1080@60",
        variable_refresh_rate = { on_demand = true },
        focus_at_startup = true,
    },
    
    -- External monitor with simple configuration
    {
        name = "HDMI-A-1",
        scale = 2.0,
        position = { x = 1920, y = 0 },
        mode = "3840x2160",  -- No refresh rate specified
        variable_refresh_rate = true,  -- Boolean shorthand
    },
    
    -- Portrait monitor with rotation
    {
        name = "DP-1",
        transform = "90",
        position = { x = 3840, y = 0 },
        mode = "2560x1440@144",
    },
    
    -- Disabled output
    {
        name = "DP-2",
        off = true,
    },
    
    -- Output with various transform options
    {
        name = "HDMI-A-2",
        transform = "flipped-180",
        position = { x = 0, y = 1080 },
    },
}
