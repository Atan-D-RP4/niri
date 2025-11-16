-- Niri Lua Configuration (EXPERIMENTAL)
-- Place this file at ~/.config/niri/niri.lua or ~/.config/niri/init.lua

-- ⚠️ WARNING: Lua configuration support is currently INCOMPLETE.

-- This example demonstrates the intended Lua configuration structure,
-- but the configuration defined below is NOT yet applied to Niri.

-- Currently, this Lua file will be executed (so niri.log() calls work),
-- but the configuration tables you define will not affect Niri's behavior.
-- You must use KDL configuration (~/.config/niri/config.kdl) for now.

-- See LUA_CONFIG_STATUS.md for details on the current limitations and
-- how to contribute to complete the Lua API implementation.

niri.log("Loading Niri Lua configuration (EXPERIMENTAL - NOT YET FUNCTIONAL)...")
niri.log("Logging from Lua works")

-- ============================================================================
-- INPUT CONFIGURATION
-- ============================================================================

-- Input device configuration
-- https://yalter.github.io/niri/Configuration:-Input

local input = {
    keyboard = {
        -- XKB configuration for keyboard layout
        xkb = {
            -- Uncomment and modify to set custom keyboard layout:
            -- layout = "us,ru",
            -- options = "grp:win_space_toggle,compose:ralt,ctrl:nocaps",
        },
        -- Enable numlock on startup
        numlock = true,
    },

    -- Touchpad settings (libinput configuration)
    touchpad = {
        enabled = true,
        tap = true,
        natural_scroll = true,
        -- accel_speed = 0.2,
        -- accel_profile = "flat",
        -- scroll_method = "two-finger",
    },

    -- Mouse settings
    mouse = {
        enabled = true,
        -- natural_scroll = false,
        -- accel_speed = 0.2,
        -- accel_profile = "flat",
    },

    -- Trackpoint settings
    trackpoint = {
        enabled = true,
        -- accel_speed = 0.2,
        -- accel_profile = "flat",
    },

    -- Uncomment to make mouse warp to center of newly focused windows
    -- warp_mouse_to_focus = true,

    -- Focus windows and outputs automatically when moving the mouse into them
    -- focus_follows_mouse = { max_scroll_amount = "0%" },
}

-- ============================================================================
-- OUTPUT CONFIGURATION
-- ============================================================================

-- Configure displays/outputs
-- https://yalter.github.io/niri/Configuration:-Outputs
-- Find output names with: niri msg outputs

-- Example output configuration (commented out by default):
--[[
local outputs = {
    {
        name = "eDP-1",  -- laptop internal display
        enabled = true,
        mode = "1920x1080@120.030",
        scale = 2.0,
        transform = "normal",
        position = { x = 1280, y = 0 },
    },
}
--]]

-- ============================================================================
-- LAYOUT CONFIGURATION
-- ============================================================================

-- Settings that influence how windows are positioned and sized
-- https://yalter.github.io/niri/Configuration:-Layout

local layout = {
    -- Set gaps around windows in logical pixels
    gaps = 16,

    -- When to center a column when changing focus
    -- Options: "never", "always", "on-overflow"
    center_focused_column = "never",

    -- Customize column width presets (fractions of output width)
    preset_column_widths = {
        0.33333,  -- 1/3
        0.5,      -- 1/2
        0.66667,  -- 2/3
    },

    -- Default column width for new windows
    default_column_width = 0.5,  -- 50% of output width

    -- Focus ring configuration
    focus_ring = {
        enabled = true,
        width = 4,
        active_color = "#7fc8ff",
        inactive_color = "#505050",
    },

    -- Border configuration (set enabled=false to disable)
    border = {
        enabled = false,
        width = 4,
        active_color = "#ffc87f",
        inactive_color = "#505050",
        urgent_color = "#9b0000",
    },

    -- Window drop shadow
    shadow = {
        enabled = false,
        softness = 30,
        spread = 5,
        offset = { x = 0, y = 5 },
        color = "#0007",
    },

    -- Struts (outer gaps, useful for panels)
    struts = {
        left = 0,
        right = 0,
        top = 0,
        bottom = 0,
    },
}

-- ============================================================================
-- STARTUP COMMANDS
-- ============================================================================

-- Spawn processes at startup
-- Note: running niri as a session supports xdg-desktop-autostart,
-- which may be more convenient to use.

local startup = {
    "waybar",
	"swaync",
    -- Add other startup commands here
}

-- ============================================================================
-- HOTKEY OVERLAY
-- ============================================================================

local hotkey_overlay = {
    -- Set skip_at_startup = true to disable the hotkey overlay at startup
    skip_at_startup = false,
}

-- ============================================================================
-- SCREENSHOT CONFIGURATION
-- ============================================================================

local screenshot = {
    -- Path for screenshots (~ expands to home directory)
    -- Supports strftime(3) format strings for date/time
    path = "~/Pictures/Screenshots/Screenshot from %Y-%m-%d %H-%M-%S.png",
    -- Set path = nil to disable saving to disk
}

-- ============================================================================
-- ANIMATION CONFIGURATION
-- ============================================================================

-- https://yalter.github.io/niri/Configuration:-Animations

local animations = {
    enabled = true,
    -- slowdown = 3.0,  -- Uncomment to slow down animations
}

-- ============================================================================
-- WINDOW RULES
-- ============================================================================

local window_rules = {
    -- Work around WezTerm's initial configure bug
    {
        match = { app_id = "^org%.wezfurlong%.wezterm$" },
        default_column_width = {},
    },

    -- Open Firefox PiP as floating
    {
        match = { app_id = "firefox$", title = "^Picture%-in%-Picture$" },
        open_floating = true,
    },

    -- Example: enable rounded corners
    -- {
    --     match = { app_id = ".*" },
    --     geometry_corner_radius = 12,
    --     clip_to_geometry = true,
    -- },
}

-- ============================================================================
-- KEYBINDINGS
-- ============================================================================

-- Keybindings configuration
-- Keys are: modifiers (Super, Ctrl, Alt, Shift) + key name
-- Find key names using: wev
-- "Mod" is Super on TTY, Alt on winit

local binds = {
    -- Show hotkey overlay
    { key = "Mod+Slash", action = "show-hotkey-overlay" },

    -- Applications
    { key = "Mod+T", action = "spawn", args = { "kitty" }, title = "Open Terminal" },
    { key = "Mod+D", action = "spawn-sh", args = { "hyde-shell rofilaunch.sh" }, title = "Run Application" },
    { key = "Mod+Alt+L", action = "spawn", args = { "hyprlock" }, title = "Lock Screen" },

    -- Volume control (with PipeWire)
    { key = "XF86AudioRaiseVolume", action = "spawn-sh", args = { "wpctl set-volume @DEFAULT_AUDIO_SINK@ 0.1+" }, allow_when_locked = true },
    { key = "XF86AudioLowerVolume", action = "spawn-sh", args = { "wpctl set-volume @DEFAULT_AUDIO_SINK@ 0.1-" }, allow_when_locked = true },
    { key = "XF86AudioMute", action = "spawn-sh", args = { "wpctl set-mute @DEFAULT_AUDIO_SINK@ toggle" }, allow_when_locked = true },
    { key = "XF86AudioMicMute", action = "spawn-sh", args = { "wpctl set-mute @DEFAULT_AUDIO_SOURCE@ toggle" }, allow_when_locked = true },

    -- Media control (with playerctl)
    { key = "XF86AudioPlay", action = "spawn-sh", args = { "playerctl play-pause" }, allow_when_locked = true },
    { key = "XF86AudioStop", action = "spawn-sh", args = { "playerctl stop" }, allow_when_locked = true },
    { key = "XF86AudioPrev", action = "spawn-sh", args = { "playerctl previous" }, allow_when_locked = true },
    { key = "XF86AudioNext", action = "spawn-sh", args = { "playerctl next" }, allow_when_locked = true },

    -- Brightness control (with brightnessctl)
    { key = "XF86MonBrightnessUp", action = "spawn", args = { "brightnessctl", "--class=backlight", "set", "+10%" }, allow_when_locked = true },
    { key = "XF86MonBrightnessDown", action = "spawn", args = { "brightnessctl", "--class=backlight", "set", "10%-" }, allow_when_locked = true },

    -- Overview
    { key = "Mod+O", action = "toggle-overview", repeat_key = false },

    -- Close window
    { key = "Mod+Q", action = "close-window", repeat_key = false },

    -- Focus navigation
    { key = "Mod+Left", action = "focus-column-left" },
    { key = "Mod+Down", action = "focus-window-down" },
    { key = "Mod+Up", action = "focus-window-up" },
    { key = "Mod+Right", action = "focus-column-right" },
    { key = "Mod+H", action = "focus-column-left" },
    { key = "Mod+J", action = "focus-window-down" },
    { key = "Mod+K", action = "focus-window-up" },
    { key = "Mod+L", action = "focus-column-right" },

    -- Window movement
    { key = "Mod+Ctrl+Left", action = "move-column-left" },
    { key = "Mod+Ctrl+Down", action = "move-window-down" },
    { key = "Mod+Ctrl+Up", action = "move-window-up" },
    { key = "Mod+Ctrl+Right", action = "move-column-right" },
    { key = "Mod+Ctrl+H", action = "move-column-left" },
    { key = "Mod+Ctrl+J", action = "move-window-down" },
    { key = "Mod+Ctrl+K", action = "move-window-up" },
    { key = "Mod+Ctrl+L", action = "move-column-right" },

    -- Column first/last
    { key = "Mod+Home", action = "focus-column-first" },
    { key = "Mod+End", action = "focus-column-last" },
    { key = "Mod+Ctrl+Home", action = "move-column-to-first" },
    { key = "Mod+Ctrl+End", action = "move-column-to-last" },

    -- Monitor focus
    { key = "Mod+Shift+Left", action = "focus-monitor-left" },
    { key = "Mod+Shift+Down", action = "focus-monitor-down" },
    { key = "Mod+Shift+Up", action = "focus-monitor-up" },
    { key = "Mod+Shift+Right", action = "focus-monitor-right" },
    { key = "Mod+Shift+H", action = "focus-monitor-left" },
    { key = "Mod+Shift+J", action = "focus-monitor-down" },
    { key = "Mod+Shift+K", action = "focus-monitor-up" },
    { key = "Mod+Shift+L", action = "focus-monitor-right" },

    -- Move column to monitor
    { key = "Mod+Shift+Ctrl+Left", action = "move-column-to-monitor-left" },
    { key = "Mod+Shift+Ctrl+Down", action = "move-column-to-monitor-down" },
    { key = "Mod+Shift+Ctrl+Up", action = "move-column-to-monitor-up" },
    { key = "Mod+Shift+Ctrl+Right", action = "move-column-to-monitor-right" },
    { key = "Mod+Shift+Ctrl+H", action = "move-column-to-monitor-left" },
    { key = "Mod+Shift+Ctrl+J", action = "move-column-to-monitor-down" },
    { key = "Mod+Shift+Ctrl+K", action = "move-column-to-monitor-up" },
    { key = "Mod+Shift+Ctrl+L", action = "move-column-to-monitor-right" },

    -- Workspace focus
    { key = "Mod+Page_Down", action = "focus-workspace-down" },
    { key = "Mod+Page_Up", action = "focus-workspace-up" },
    { key = "Mod+U", action = "focus-workspace-down" },
    { key = "Mod+I", action = "focus-workspace-up" },

    -- Move column to workspace
    { key = "Mod+Ctrl+Page_Down", action = "move-column-to-workspace-down" },
    { key = "Mod+Ctrl+Page_Up", action = "move-column-to-workspace-up" },
    { key = "Mod+Ctrl+U", action = "move-column-to-workspace-down" },
    { key = "Mod+Ctrl+I", action = "move-column-to-workspace-up" },

    -- Move workspace
    { key = "Mod+Shift+Page_Down", action = "move-workspace-down" },
    { key = "Mod+Shift+Page_Up", action = "move-workspace-up" },
    { key = "Mod+Shift+U", action = "move-workspace-down" },
    { key = "Mod+Shift+I", action = "move-workspace-up" },

    -- Workspace mouse wheel scroll
    { key = "Mod+WheelScrollDown", action = "focus-workspace-down", cooldown = 150 },
    { key = "Mod+WheelScrollUp", action = "focus-workspace-up", cooldown = 150 },
    { key = "Mod+Ctrl+WheelScrollDown", action = "move-column-to-workspace-down", cooldown = 150 },
    { key = "Mod+Ctrl+WheelScrollUp", action = "move-column-to-workspace-up", cooldown = 150 },

    -- Column mouse wheel scroll
    { key = "Mod+WheelScrollRight", action = "focus-column-right" },
    { key = "Mod+WheelScrollLeft", action = "focus-column-left" },
    { key = "Mod+Ctrl+WheelScrollRight", action = "move-column-right" },
    { key = "Mod+Ctrl+WheelScrollLeft", action = "move-column-left" },

    -- Shift wheel for horizontal scroll
    { key = "Mod+Shift+WheelScrollDown", action = "focus-column-right" },
    { key = "Mod+Shift+WheelScrollUp", action = "focus-column-left" },
    { key = "Mod+Ctrl+Shift+WheelScrollDown", action = "move-column-right" },
    { key = "Mod+Ctrl+Shift+WheelScrollUp", action = "move-column-left" },

    -- Workspace switching by number
    { key = "Mod+1", action = "focus-workspace", args = { 1 } },
    { key = "Mod+2", action = "focus-workspace", args = { 2 } },
    { key = "Mod+3", action = "focus-workspace", args = { 3 } },
    { key = "Mod+4", action = "focus-workspace", args = { 4 } },
    { key = "Mod+5", action = "focus-workspace", args = { 5 } },
    { key = "Mod+6", action = "focus-workspace", args = { 6 } },
    { key = "Mod+7", action = "focus-workspace", args = { 7 } },
    { key = "Mod+8", action = "focus-workspace", args = { 8 } },
    { key = "Mod+9", action = "focus-workspace", args = { 9 } },

    -- Move column to workspace by number
    { key = "Mod+Ctrl+1", action = "move-column-to-workspace", args = { 1 } },
    { key = "Mod+Ctrl+2", action = "move-column-to-workspace", args = { 2 } },
    { key = "Mod+Ctrl+3", action = "move-column-to-workspace", args = { 3 } },
    { key = "Mod+Ctrl+4", action = "move-column-to-workspace", args = { 4 } },
    { key = "Mod+Ctrl+5", action = "move-column-to-workspace", args = { 5 } },
    { key = "Mod+Ctrl+6", action = "move-column-to-workspace", args = { 6 } },
    { key = "Mod+Ctrl+7", action = "move-column-to-workspace", args = { 7 } },
    { key = "Mod+Ctrl+8", action = "move-column-to-workspace", args = { 8 } },
    { key = "Mod+Ctrl+9", action = "move-column-to-workspace", args = { 9 } },

    -- Column management
    { key = "Mod+BracketLeft", action = "consume-or-expel-window-left" },
    { key = "Mod+BracketRight", action = "consume-or-expel-window-right" },
    { key = "Mod+Comma", action = "consume-window-into-column" },
    { key = "Mod+Period", action = "expel-window-from-column" },

    -- Column width presets
    { key = "Mod+R", action = "switch-preset-column-width" },
    { key = "Mod+Shift+R", action = "switch-preset-window-height" },
    { key = "Mod+Ctrl+R", action = "reset-window-height" },

    -- Column and window sizing
    { key = "Mod+F", action = "maximize-column" },
    { key = "Mod+Shift+F", action = "fullscreen-window", repeat_key = false },
    { key = "Mod+Ctrl+F", action = "expand-column-to-available-width" },

    -- Column centering
    { key = "Mod+C", action = "center-column" },
    { key = "Mod+Ctrl+C", action = "center-visible-columns" },

    -- Fine width/height adjustments
    { key = "Mod+Minus", action = "set-column-width", args = { "-10%" } },
    { key = "Mod+Equal", action = "set-column-width", args = { "+10%" } },
    { key = "Mod+Shift+Minus", action = "set-window-height", args = { "-10%" } },
    { key = "Mod+Shift+Equal", action = "set-window-height", args = { "+10%" } },

    -- Floating toggle
    { key = "Mod+V", action = "toggle-window-floating" },
    { key = "Mod+Shift+V", action = "switch-focus-between-floating-and-tiling" },

    -- Tabbed column display mode
    { key = "Mod+W", action = "toggle-column-tabbed-display" },

    -- Screenshots
    { key = "Print", action = "screenshot" },
    { key = "Ctrl+Print", action = "screenshot-screen" },
    { key = "Alt+Print", action = "screenshot-window" },

    -- Keyboard shortcuts inhibitor toggle
    { key = "Mod+Escape", action = "toggle-keyboard-shortcuts-inhibit", allow_inhibiting = false },

    -- Exit/shutdown
    { key = "Mod+Shift+E", action = "quit" },

    -- Power management
    { key = "Mod+Shift+P", action = "power-off-monitors" },
}

-- ============================================================================
-- TIER 1 LUA FOUNDATION STATUS
-- ============================================================================
-- This configuration demonstrates Tier 1 features of the Niri Lua API:

-- ✓ SUPPORTED IN TIER 1:
--   - Module loading system (require support)
--   - Plugin system for extensibility
--   - Event emitter for event-driven configuration
--   - Hot reload for live configuration updates
--   - Configuration tables (input, layout, binds, etc.)

-- 🔄 IN DEVELOPMENT (TIER 2):
--   - Keybinding application through Niri API
--   - Startup command execution (niri.spawn)
--   - Direct configuration API for input/layout/animations
--   - Window rules and filter matching
--   - Gesture configuration

-- HOW IT WORKS:
-- The configuration tables defined above (input, layout, animations, etc.)
-- are automatically parsed and applied by Niri's configuration system when
-- this Lua file is loaded. Key bindings, window rules, and startup commands
-- defined in the tables will be converted to appropriate Rust structures
-- and applied to the running Niri instance.
-- ============================================================================

-- Configuration is applied automatically during loading
niri.log("Niri Lua configuration loaded successfully!")

-- Return configuration for potential programmatic access
return {
    input = input,
    layout = layout,
    animations = animations,
    hotkey_overlay = hotkey_overlay,
    screenshot = screenshot,
    startup = startup,
    window_rules = window_rules,
    binds = binds,
}
