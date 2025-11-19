-- Test layout configuration
-- This file demonstrates all supported layout configuration options

layout = {
    -- Basic layout settings
    gaps = 12.0,
    always_center_single_column = true,
    empty_workspace_above_first = false,
    
    -- Column and display settings
    center_focused_column = "on-overflow",  -- Options: "never", "always", "on-overflow"
    default_column_display = "normal",      -- Options: "normal", "tabbed"
    
    -- Background color (supports hex strings and RGBA tables)
    background_color = "#1e1e2e",  -- Catppuccin Mocha base color
    -- Or use RGBA: background_color = { r = 0.118, g = 0.118, b = 0.180, a = 1.0 }
    
    -- Struts (reserved space at screen edges)
    struts = {
        left = 0.0,
        right = 0.0,
        top = 0.0,
        bottom = 0.0,
    },
    
    -- Preset column widths (for quick window resizing)
    -- Can be proportions (0.0-1.0) or fixed pixel values
    preset_column_widths = {
        0.33,                      -- Direct proportion value
        { proportion = 0.5 },      -- Explicit proportion format
        { proportion = 0.67 },
        { fixed = 1200 },          -- Fixed pixel width
    },
    
    -- Default column width
    default_column_width = { proportion = 0.5 },
    
    -- Preset window heights
    preset_window_heights = {
        0.25,
        { proportion = 0.33 },
        { proportion = 0.5 },
        { proportion = 0.67 },
        { fixed = 800 },
    },
    
    -- Focus ring (visual indicator around focused window)
    focus_ring = {
        off = false,
        width = 4.0,
        active_color = "#89b4fa",    -- Catppuccin blue (hex format)
        inactive_color = { r = 0.3, g = 0.3, b = 0.3, a = 1.0 },  -- RGBA format
        urgent_color = "#f38ba8",    -- Catppuccin red
    },
    
    -- Border (window border, different from focus ring)
    border = {
        on = false,  -- Enable border (default is off)
        width = 2.0,
        active_color = "#fab387",    -- Catppuccin peach
        inactive_color = "#313244",  -- Catppuccin surface0
        urgent_color = "#f38ba8",    -- Catppuccin red
    },
    
    -- Insert hint (visual indicator when inserting windows)
    insert_hint = {
        on = true,
        color = "#89b4fa80",  -- Blue with transparency (hex with alpha)
        -- Or use RGBA: color = { r = 0.537, g = 0.706, b = 0.980, a = 0.5 }
    },
    
    -- Shadow (drop shadow for windows)
    shadow = {
        on = true,
        softness = 30.0,
        spread = 5.0,
        draw_behind_window = false,
        color = "#00000080",  -- Black with 50% transparency
        -- Optional inactive color (if not set, uses color)
        -- inactive_color = "#00000040",
        offset = {
            x = 0.0,
            y = 5.0,
        },
    },
}

-- Return the configuration
return {
    layout = layout
}
