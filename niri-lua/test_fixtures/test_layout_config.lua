-- Test Layout Configuration for Niri (v2 API)
-- This file tests all layout configuration options

niri.utils.log("Loading layout configuration test")

-- Configure layout using v2 proxy API
niri.config.layout = {
    -- Basic layout settings
    gaps = 12,
    always_center_single_column = true,
    empty_workspace_above_first = false,

    -- Column and display settings
    center_focused_column = "on-overflow", -- Options: "never", "always", "on-overflow"
    default_column_display = "normal", -- Options: "normal", "tabbed"

    -- Background color (supports hex strings)
    background_color = "#1e1e2e", -- Catppuccin Mocha base color

    -- Struts (reserved space at screen edges)
    struts = {
        left = 0,
        right = 0,
        top = 0,
        bottom = 0,
    },

    -- Preset column widths (for quick window resizing)
    preset_column_widths = {
        { proportion = 0.33 },
        { proportion = 0.5 },
        { proportion = 0.67 },
        { fixed = 1200 },
    },

    -- Default column width
    default_column_width = { proportion = 0.5 },

    -- Preset window heights
    preset_window_heights = {
        { proportion = 0.25 },
        { proportion = 0.33 },
        { proportion = 0.5 },
        { proportion = 0.67 },
        { fixed = 800 },
    },

    -- Focus ring (visual indicator around focused window)
    focus_ring = {
        width = 4,
        active_color = "#89b4fa", -- Catppuccin blue
        inactive_color = "#505050",
        urgent_color = "#f38ba8", -- Catppuccin red
    },

    -- Border (window border, different from focus ring)
    border = {
        off = true, -- Disabled when using focus ring
        width = 2,
        active_color = "#fab387", -- Catppuccin peach
        inactive_color = "#313244", -- Catppuccin surface0
        urgent_color = "#f38ba8",
    },

    -- Insert hint (visual indicator when inserting windows)
    insert_hint = {
        color = "#89b4fa80", -- Blue with transparency
    },

    -- Shadow (drop shadow for windows)
    shadow = {
        softness = 30,
        spread = 5,
        color = "#00000080",
        offset = {
            x = 0,
            y = 5,
        },
    },
}

-- Apply the configuration
niri.config:apply()

niri.utils.log("Layout configuration loaded successfully")
