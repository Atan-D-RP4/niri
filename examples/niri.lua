-- ============================================================================
-- Niri Lua Configuration Example
-- ============================================================================
-- This is the comprehensive example Lua configuration for Niri.

-- To use this configuration:
-- 1. Copy this file to: ~/.config/niri/niri.lua (or init.lua)
-- 2. Uncomment and modify the settings you want to change
-- 3. Restart niri to apply changes

-- This configuration demonstrates ALL implemented Lua API features:
-- - Tier 1: Module System (100% complete)
-- - Tier 2: Configuration API (100% complete)
-- - Tier 3: Runtime State Access (100% complete)
-- - Full keybinding converter support
-- - Event system integration (in progress)

-- Key Feature: niri.apply_config()
-- This example uses niri.apply_config() which allows you to:
-- 1. Build your config using local variables and functions
-- 2. Apply it to niri with niri.apply_config({ ... })
-- 3. Continue with additional Lua scripting after config is applied
-- 4. No need for async callbacks - everything runs sequentially!

-- Documentation: https://yalter.github.io/niri/
-- ============================================================================

-- ============================================================================
-- INPUT CONFIGURATION
-- ============================================================================
-- Configure keyboard, mouse, touchpad, and other input devices.
-- All settings here match the KDL config format.

local input = {
	-- Keyboard configuration
	keyboard = {
		-- XKB keyboard layout settings
		xkb = {
			-- Keyboard layout (e.g., "us", "us,ru", "de")
			layout = "us",

			-- Layout variant (e.g., "dvorak", "colemak")
			-- variant = "intl",

			-- XKB rules file
			-- rules = "evdev",

			-- Keyboard model
			-- model = "pc105",

			-- XKB options (comma-separated)
			-- Common options:
			-- - "grp:win_space_toggle" - Switch layouts with Win+Space
			-- - "ctrl:nocaps" - Make Caps Lock act as Ctrl
			-- - "compose:ralt" - Right Alt as Compose key
			options = "ctrl:nocaps",
		},

		-- Key repeat delay in milliseconds (default: 600)
		-- repeat_delay = 600,

		-- Key repeat rate in characters per second (default: 25)
		-- repeat_rate = 25,

		-- Enable numlock on startup
		numlock = false,
	},

	-- Mouse configuration
	mouse = {
		-- Acceleration speed from -1.0 to 1.0 (default: 0.0)
		-- accel_speed = 0.0,

		-- Acceleration profile: "adaptive" or "flat"
		-- accel_profile = "adaptive",

		-- Natural scrolling (reverse scroll direction)
		-- natural_scroll = false,

		-- Scroll method: "no-scroll", "two-finger", "edge", "on-button-down"
		-- scroll_method = "no-scroll",
	},

	-- Touchpad configuration
	touchpad = {
		-- Acceleration speed from -1.0 to 1.0
		-- accel_speed = 0.0,

		-- Acceleration profile: "adaptive" or "flat"
		-- accel_profile = "adaptive",

		-- Tap to click
		tap = true,

		-- Tap button mapping: "left-right-middle" or "left-middle-right"
		-- tap_button_map = "left-right-middle",

		-- Natural scrolling (reverse scroll direction)
		natural_scroll = true,

		-- Scroll method: "no-scroll", "two-finger", "edge", "on-button-down"
		-- scroll_method = "two-finger",

		-- Disable while typing
		-- dwt = true,

		-- Disable while trackpoint is in use
		-- dwtp = true,

		-- Drag and drop
		-- drag = true,

		-- Drag lock
		-- drag_lock = false,

		-- Disable touchpad when external mouse is connected
		-- disabled_on_external_mouse = false,
	},

	-- Trackpoint configuration
	trackpoint = {
		-- accel_speed = 0.0,
		-- accel_profile = "adaptive",
		-- natural_scroll = false,
		-- scroll_method = "on-button-down",
		-- scroll_button = 273,  -- BTN_MIDDLE
		-- scroll_button_lock = false,
		-- middle_emulation = false,
	},

	-- Warp mouse cursor to center of newly focused windows
	-- warp_mouse_to_focus = false,

	-- Focus windows when moving mouse into them
	-- focus_follows_mouse = {
	--     -- Maximum scroll amount (as percentage) to focus window
	--     -- Set to "0%" to only focus windows fully on screen
	--     max_scroll_amount = "0%",
	-- },
}

-- ============================================================================
-- LAYOUT CONFIGURATION
-- ============================================================================
-- Control window positioning, sizing, borders, shadows, and gaps.

local layout = {
	-- Gaps around windows in logical pixels
	gaps = 16,

	-- Background color for empty workspace areas
	-- Supports: CSS colors ("red"), hex ("#rrggbb"), rgb(), rgba(), hsl()
	-- background_color = "transparent",

	-- When to center focused column: "never", "always", "on-overflow"
	-- - "never": Default behavior, keeps column at edge when off-screen
	-- - "always": Always centers the focused column
	-- - "on-overflow": Center if it doesn't fit with previous column
	center_focused_column = "never",

	-- Always center a single column on the screen
	-- always_center_single_column = false,

	-- Keep an empty workspace above the first real workspace
	-- empty_workspace_above_first = false,

	-- Default column display mode: "normal" or "tabbed"
	-- default_column_display = "normal",

	-- Preset column widths cycled with Mod+R (switch-preset-column-width)
	-- Can use "proportion" (fraction of output width) or "fixed" (pixels)
	preset_column_widths = {
		{ proportion = 1.0 / 3.0 }, -- 33.33%
		{ proportion = 1.0 / 2.0 }, -- 50%
		{ proportion = 2.0 / 3.0 }, -- 66.67%
		-- { fixed = 1920 },       -- Fixed pixel width
	},

	-- Default width for new windows
	-- Empty {} lets windows decide their own initial width
	default_column_width = { proportion = 0.5 },
	-- default_column_width = {},

	-- Preset window heights cycled with Mod+Shift+R (switch-preset-window-height)
	preset_window_heights = {
		{ proportion = 1.0 / 3.0 },
		{ proportion = 1.0 / 2.0 },
		{ proportion = 2.0 / 3.0 },
	},

	-- Focus ring - highlights the active window
	focus_ring = {
		-- Disable focus ring
		-- off = false,

		-- Width in logical pixels
		width = 4,

		-- Colors for different states
		-- Active monitor's focused window
		active_color = "#7fc8ff",

		-- Inactive monitor's focused window
		inactive_color = "#505050",

		-- Window requesting attention
		-- urgent_color = "#ff0000",

		-- Gradients (take precedence over solid colors)
		-- active_gradient = {
		--     from = "#80c8ff",
		--     to = "#c7ff7f",
		--     angle = 45,  -- Optional, defaults to 180 (top-to-bottom)
		--     relative_to = "window",  -- "window" or "workspace-view"
		--     in_color_space = "srgb",  -- "srgb", "oklch", "oklch longer hue", etc.
		-- },
	},

	-- Border - always visible outline around windows
	border = {
		-- Disable border (if using focus ring, usually disable one or the other)
		off = true,

		-- Width in logical pixels
		width = 4,

		-- Colors
		active_color = "#ffc87f",
		inactive_color = "#505050",
		urgent_color = "#9b0000",

		-- Gradients work the same as focus_ring
		-- active_gradient = { from = "#ffc87f", to = "#ff7f7f", angle = 90 },
	},

	-- Drop shadow for windows
	shadow = {
		-- Enable shadows (disabled by default)
		-- on = false,

		-- Draw shadow behind window (fixes CSD rounded corners artifacts)
		-- draw_behind_window = false,

		-- Shadow blur radius in logical pixels
		softness = 30,

		-- Shadow expansion
		spread = 5,

		-- Shadow offset
		offset = {
			x = 0,
			y = 5,
		},

		-- Shadow color (supports alpha channel)
		color = "#0007", -- Black with ~27% opacity
	},

	-- Tab indicator - shows active tab in tabbed column display
	tab_indicator = {
		-- off = false,
		-- width = 4,
		-- active_color = "#7fc8ff",
		-- inactive_color = "#505050",
		-- urgent_color = "#ff0000",
	},

	-- Insert hint - shows where window will be inserted when dragging
	insert_hint = {
		-- off = false,
		-- color = "#7fc8ff80",  -- Semi-transparent blue
	},

	-- Struts - reserved space at screen edges (like outer gaps)
	-- Measured in logical pixels
	struts = {
		-- left = 0,
		-- right = 0,
		-- top = 0,
		-- bottom = 0,
	},
}

-- ============================================================================
-- OUTPUT CONFIGURATION
-- ============================================================================
-- Configure monitor settings (resolution, scale, position).
-- Get output names with: niri msg outputs

local outputs = {
	-- Example: built-in laptop display
	["eDP-1"] = {
	    -- Disable this output
	    -- off = false,

	    -- Mode: "WIDTHxHEIGHT" or "WIDTHxHEIGHT@REFRESH"
	    -- If omitted, niri picks the best mode automatically
	    mode = "1920x1080@60",

	    -- Scale factor (1.0 = 100%, 1.5 = 150%, 2.0 = 200%)
	    scale = 1.5,

	    -- Transform: "normal", "90", "180", "270",
	    --           "flipped", "flipped-90", "flipped-180", "flipped-270"
	    -- transform = "normal",

	    -- Position in global coordinate space (in logical pixels)
	    -- position = { x = 0, y = 0 },

	    -- Variable refresh rate (VRR/FreeSync/G-Sync)
	    -- vrr = false,
	},

	-- Example: external monitor
	-- ["HDMI-A-1"] = {
	--     mode = "3840x2160@60",
	--     scale = 1.5,
	--     position = { x = 1920, y = 0 },
	-- },
}

-- ============================================================================
-- CURSOR CONFIGURATION
-- ============================================================================
-- Configure cursor theme, size, and hiding behavior.

local cursor = {
	-- Xcursor theme name
	-- xcursor_theme = "Adwaita",

	-- Cursor size in pixels
	xcursor_size = 24,

	-- Hide cursor while typing
	hide_when_typing = false,

	-- Hide cursor after N milliseconds of inactivity
	-- hide_after_inactive_ms = 5000,
}

-- ============================================================================
-- GESTURES CONFIGURATION
-- ============================================================================
-- Configure touchpad gestures and hot corners.

local gestures = {
	-- Drag-drop edge view scrolling (scroll view when dragging near edge)
	-- drag_drop_edge_view_scroll = {
	--     trigger_width = 32,  -- Pixels from edge to trigger
	--     delay_ms = 500,      -- Delay before scrolling starts
	--     max_speed = 1000,    -- Maximum scroll speed
	-- },

	-- Drag-drop edge workspace switching (switch workspace when dragging near edge)
	-- drag_drop_edge_workspace_switch = {
	--     trigger_height = 32,
	--     delay_ms = 500,
	--     max_speed = 1000,
	-- },

	-- Hot corners - perform actions when cursor touches screen corners
	-- hot_corners = {
	--     off = false,
	--     top_left = "toggle-overview",
	--     top_right = nil,
	--     bottom_left = nil,
	--     bottom_right = nil,
	-- },
}

-- ============================================================================
-- RECENT WINDOWS (MRU) CONFIGURATION
-- ============================================================================
-- Configure the recent windows overlay (Alt+Tab style switcher).

local recent_windows = {
	-- Enable recent windows feature
	on = true,

	-- Delay before showing overlay in milliseconds (default: 150)
	open_delay_ms = 150,

	-- Highlight configuration
	highlight = {
		-- Default colors (gray for active, reddish for urgent)
		active_color = "#999999", -- Default: Color::new_unpremul(0.6, 0.6, 0.6, 1.)
		urgent_color = "#ff9999", -- Default: Color::new_unpremul(1., 0.6, 0.6, 1.)
		padding = 30, -- Default padding
		corner_radius = 0, -- Default: no rounding
	},

	-- Window preview configuration
	previews = {
		max_height = 480, -- Default maximum height
		max_scale = 0.5, -- Default maximum scale
	},
}

-- ============================================================================
-- OVERVIEW CONFIGURATION
-- ============================================================================
-- Configure the workspace overview mode.

local overview = {
	-- Zoom level (1.0 = no zoom, < 1.0 = zoom out)
	zoom = 0.5,

	-- Backdrop color
	backdrop_color = "#00000080", -- Semi-transparent black

	-- Shadow around workspace thumbnails
	workspace_shadow = {
		-- off = false,
		softness = 30,
		spread = 5,
		offset = { x = 0, y = 5 },
		color = "#0007",
	},
}

-- ============================================================================
-- ANIMATIONS CONFIGURATION
-- ============================================================================
-- Configure animation speeds and curves.

local animations = {
	-- Disable all animations
	-- off = false,

	-- Global slowdown multiplier (>1.0 = slower, <1.0 = faster)
	-- slowdown = 1.0,

	-- Individual animation settings
	-- Each can have: off, duration_ms, curve (spring or easing)

	-- workspace_switch = {
	--     off = false,
	--     -- duration_ms = 250,
	--     -- Spring curve (natural physics-based)
	--     -- curve = {
	--     --     spring = {
	--     --         damping_ratio = 1.0,
	--     --         stiffness = 1000,
	--     --         epsilon = 0.0001,
	--     --     }
	--     -- },
	--     -- Easing curve
	--     -- curve = { easing = "ease-out-cubic" },
	-- },

	-- window_open = { off = false },
	-- window_close = { off = false },
	-- horizontal_view_movement = { off = false },
	-- window_movement = { off = false },
	-- window_resize = { off = false },
	-- config_notification_open_close = { off = false },
	-- exit_confirmation_open_close = { off = false },
	-- screenshot_ui_open = { off = false },
	-- overview_open_close = { off = false },
	-- recent_windows_close = { off = false },
}

-- ============================================================================
-- CLIPBOARD CONFIGURATION
-- ============================================================================
-- Configure clipboard behavior.

local clipboard = {
	-- Disable primary selection (middle-click paste)
	disable_primary = false,
}

-- ============================================================================
-- HOTKEY OVERLAY CONFIGURATION
-- ============================================================================
-- Configure the hotkey help overlay (shown with Mod+Shift+/).

local hotkey_overlay = {
	-- Skip showing overlay at startup
	skip_at_startup = false,

	-- Hide keybindings without hotkey-overlay-title
	-- hide_not_bound = false,
}

-- ============================================================================
-- CONFIG NOTIFICATION CONFIGURATION
-- ============================================================================
-- Configure error notifications when config reload fails.

local config_notification = {
	-- Disable failed config notifications
	disable_failed = false,
}

-- ============================================================================
-- DEBUG CONFIGURATION
-- ============================================================================
-- Advanced debugging and performance options.

local debug = {
	-- disable_direct_scanout = false,
	-- enable_overlay_planes = false,
	-- render_drm_device = "/dev/dri/renderD128",
	-- disable_cursor_plane = false,
	-- wait_for_frame_completion_before_queueing = false,
	-- emulate_zero_presentation_time = false,
	-- disable_drm_compositing = false,
	-- more_pixel_shader_invocations = false,
	-- disable_pipewire_server = false,
	-- disable_pipewire_capture_cursor = false,
	-- dbus_interfaces_in_non_session_instances = false,
	-- block_out_from = nil,
	-- damage_tracking = "auto",
}

-- ============================================================================
-- XWAYLAND SATELLITE CONFIGURATION
-- ============================================================================
-- Configure Xwayland support via xwayland-satellite.

local xwayland_satellite = {
	-- Disable Xwayland
	off = true,

	-- Path to xwayland-satellite binary
	-- path = "xwayland-satellite",
}

-- ============================================================================
-- SCREENSHOT CONFIGURATION
-- ============================================================================
-- Configure screenshot save path.

local screenshot = {
	-- Path with strftime formatting
	-- Set to nil/null to disable saving to disk
	path = "~/Pictures/Screenshots/Screenshot from %Y-%m-%d %H-%M-%S.png",
}

-- ============================================================================
-- KEYBINDINGS
-- ============================================================================
-- Define keyboard and mouse bindings.
-- Find XKB key names with: wev or xev
-- "Mod" = Super (TTY) or Alt (winit window)

local binds = {
	-- ====================
	-- SYSTEM & COMPOSITOR
	-- ====================

	-- Show hotkey overlay
	{ key = "Mod+Shift+Slash", action = "show-hotkey-overlay" },

	-- Quit with confirmation
	{ key = "Mod+Shift+E", action = "quit" },
	{ key = "Ctrl+Alt+Delete", action = "quit" },

	-- Power off monitors (wake with any input)
	{ key = "Mod+Shift+P", action = "power-off-monitors" },

	-- Toggle keyboard shortcuts inhibit (for VNC/remote desktop apps)
	{ key = "Mod+Escape", action = "toggle-keyboard-shortcuts-inhibit" },

	-- ====================
	-- APPLICATION LAUNCHING
	-- ====================

	-- Terminal
	{
		key = "Mod+T",
		action = "spawn",
		args = { "kitty" },
		-- Optional: title shown in hotkey overlay
		-- hotkey_overlay_title = "Open a Terminal: alacritty",
	},

	-- Application launcher
	{ key = "Mod+D", action = "spawn-sh", args = { "hyde-shell rofilaunch.sh" } },

	-- Screen locker
	{ key = "Super+Alt+L", action = "spawn", args = { "hyde-shell hyprlock.sh" } },

	-- Shell command example (with pipes, variables, etc.)
	-- {
	--     key = "Super+Alt+S",
	--     action = "spawn-sh",
	--     args = { "pkill orca || exec orca" },
	--     -- allow_when_locked = true,
	-- },

	-- ====================
	-- MEDIA KEYS (example with wpctl for PipeWire)
	-- ====================

	-- Volume
	-- { key = "XF86AudioRaiseVolume", action = "spawn-sh", args = { "wpctl set-volume @DEFAULT_AUDIO_SINK@ 0.1+" } },
	-- { key = "XF86AudioLowerVolume", action = "spawn-sh", args = { "wpctl set-volume @DEFAULT_AUDIO_SINK@ 0.1-" } },
	-- { key = "XF86AudioMute", action = "spawn-sh", args = { "wpctl set-mute @DEFAULT_AUDIO_SINK@ toggle" } },
	-- { key = "XF86AudioMicMute", action = "spawn-sh", args = { "wpctl set-mute @DEFAULT_AUDIO_SOURCE@ toggle" } },

	-- Media playback (with playerctl)
	-- { key = "XF86AudioPlay", action = "spawn-sh", args = { "playerctl play-pause" } },
	-- { key = "XF86AudioStop", action = "spawn-sh", args = { "playerctl stop" } },
	-- { key = "XF86AudioPrev", action = "spawn-sh", args = { "playerctl previous" } },
	-- { key = "XF86AudioNext", action = "spawn-sh", args = { "playerctl next" } },

	-- Brightness (with brightnessctl)
	-- { key = "XF86MonBrightnessUp", action = "spawn", args = { "brightnessctl", "--class=backlight", "set", "+10%" } },
	-- { key = "XF86MonBrightnessDown", action = "spawn", args = { "brightnessctl", "--class=backlight", "set", "10%-" } },

	-- ====================
	-- WINDOW MANAGEMENT
	-- ====================

	-- Close window
	{ key = "Mod+Q", action = "close-window" },

	-- Fullscreen
	{ key = "Mod+Shift+F", action = "fullscreen-window" },

	-- Maximize column
	{ key = "Mod+F", action = "maximize-column" },

	-- Expand column to fill available width
	{ key = "Mod+Ctrl+F", action = "expand-column-to-available-width" },

	-- Center column
	{ key = "Mod+C", action = "center-column" },

	-- Center all visible columns
	{ key = "Mod+Ctrl+C", action = "center-visible-columns" },

	-- Toggle floating/tiling
	{ key = "Mod+V", action = "toggle-window-floating" },

	-- Switch focus between floating and tiling
	{ key = "Mod+Shift+V", action = "switch-focus-between-floating-and-tiling" },

	-- Toggle tabbed column display
	{ key = "Mod+W", action = "toggle-column-tabbed-display" },

	-- ====================
	-- WINDOW FOCUS
	-- ====================

	-- Focus column left/right
	{ key = "Mod+Left", action = "focus-column-left" },
	{ key = "Mod+Right", action = "focus-column-right" },
	{ key = "Mod+H", action = "focus-column-left" },
	{ key = "Mod+L", action = "focus-column-right" },

	-- Focus window up/down
	{ key = "Mod+Down", action = "focus-window-down" },
	{ key = "Mod+Up", action = "focus-window-up" },
	{ key = "Mod+J", action = "focus-window-down" },
	{ key = "Mod+K", action = "focus-window-up" },

	-- Focus first/last column
	{ key = "Mod+Home", action = "focus-column-first" },
	{ key = "Mod+End", action = "focus-column-last" },

	-- Focus across workspaces
	-- { key = "Mod+J", action = "focus-window-or-workspace-down" },
	-- { key = "Mod+K", action = "focus-window-or-workspace-up" },

	-- ====================
	-- WINDOW MOVEMENT
	-- ====================

	-- Move column left/right
	{ key = "Mod+Ctrl+Left", action = "move-column-left" },
	{ key = "Mod+Ctrl+Right", action = "move-column-right" },
	{ key = "Mod+Ctrl+H", action = "move-column-left" },
	{ key = "Mod+Ctrl+L", action = "move-column-right" },

	-- Move window up/down
	{ key = "Mod+Ctrl+Down", action = "move-window-down" },
	{ key = "Mod+Ctrl+Up", action = "move-window-up" },
	{ key = "Mod+Ctrl+J", action = "move-window-down" },
	{ key = "Mod+Ctrl+K", action = "move-window-up" },

	-- Move to first/last
	{ key = "Mod+Ctrl+Home", action = "move-column-to-first" },
	{ key = "Mod+Ctrl+End", action = "move-column-to-last" },

	-- Consume/expel windows
	{ key = "Mod+BracketLeft", action = "consume-or-expel-window-left" },
	{ key = "Mod+BracketRight", action = "consume-or-expel-window-right" },
	{ key = "Mod+Comma", action = "consume-window-into-column" },
	{ key = "Mod+Period", action = "expel-window-from-column" },

	-- ====================
	-- MONITOR FOCUS
	-- ====================

	{ key = "Mod+Shift+Left", action = "focus-monitor-left" },
	{ key = "Mod+Shift+Right", action = "focus-monitor-right" },
	{ key = "Mod+Shift+Down", action = "focus-monitor-down" },
	{ key = "Mod+Shift+Up", action = "focus-monitor-up" },
	{ key = "Mod+Shift+H", action = "focus-monitor-left" },
	{ key = "Mod+Shift+L", action = "focus-monitor-right" },
	{ key = "Mod+Shift+J", action = "focus-monitor-down" },
	{ key = "Mod+Shift+K", action = "focus-monitor-up" },

	-- ====================
	-- MONITOR MOVEMENT
	-- ====================

	{ key = "Mod+Shift+Ctrl+Left", action = "move-column-to-monitor-left" },
	{ key = "Mod+Shift+Ctrl+Right", action = "move-column-to-monitor-right" },
	{ key = "Mod+Shift+Ctrl+Down", action = "move-column-to-monitor-down" },
	{ key = "Mod+Shift+Ctrl+Up", action = "move-column-to-monitor-up" },
	{ key = "Mod+Shift+Ctrl+H", action = "move-column-to-monitor-left" },
	{ key = "Mod+Shift+Ctrl+L", action = "move-column-to-monitor-right" },
	{ key = "Mod+Shift+Ctrl+J", action = "move-column-to-monitor-down" },
	{ key = "Mod+Shift+Ctrl+K", action = "move-column-to-monitor-up" },

	-- ====================
	-- WORKSPACE NAVIGATION
	-- ====================

	-- Cycle workspaces
	{ key = "Mod+Page_Down", action = "focus-workspace-down" },
	{ key = "Mod+Page_Up", action = "focus-workspace-up" },
	{ key = "Mod+U", action = "focus-workspace-down" },
	{ key = "Mod+I", action = "focus-workspace-up" },

	-- Focus specific workspace by index
	{ key = "Mod+1", action = "focus-workspace", args = { "1" } },
	{ key = "Mod+2", action = "focus-workspace", args = { "2" } },
	{ key = "Mod+3", action = "focus-workspace", args = { "3" } },
	{ key = "Mod+4", action = "focus-workspace", args = { "4" } },
	{ key = "Mod+5", action = "focus-workspace", args = { "5" } },
	{ key = "Mod+6", action = "focus-workspace", args = { "6" } },
	{ key = "Mod+7", action = "focus-workspace", args = { "7" } },
	{ key = "Mod+8", action = "focus-workspace", args = { "8" } },
	{ key = "Mod+9", action = "focus-workspace", args = { "9" } },

	-- Previous workspace
	-- { key = "Mod+Tab", action = "focus-workspace-previous" },

	-- ====================
	-- WORKSPACE MOVEMENT
	-- ====================

	-- Move column to workspace
	{ key = "Mod+Ctrl+Page_Down", action = "move-column-to-workspace-down" },
	{ key = "Mod+Ctrl+Page_Up", action = "move-column-to-workspace-up" },
	{ key = "Mod+Ctrl+U", action = "move-column-to-workspace-down" },
	{ key = "Mod+Ctrl+I", action = "move-column-to-workspace-up" },

	-- Move column to specific workspace
	{ key = "Mod+Ctrl+1", action = "move-column-to-workspace", args = { "1" } },
	{ key = "Mod+Ctrl+2", action = "move-column-to-workspace", args = { "2" } },
	{ key = "Mod+Ctrl+3", action = "move-column-to-workspace", args = { "3" } },
	{ key = "Mod+Ctrl+4", action = "move-column-to-workspace", args = { "4" } },
	{ key = "Mod+Ctrl+5", action = "move-column-to-workspace", args = { "5" } },
	{ key = "Mod+Ctrl+6", action = "move-column-to-workspace", args = { "6" } },
	{ key = "Mod+Ctrl+7", action = "move-column-to-workspace", args = { "7" } },
	{ key = "Mod+Ctrl+8", action = "move-column-to-workspace", args = { "8" } },
	{ key = "Mod+Ctrl+9", action = "move-column-to-workspace", args = { "9" } },

	-- Move workspace itself
	{ key = "Mod+Shift+Page_Down", action = "move-workspace-down" },
	{ key = "Mod+Shift+Page_Up", action = "move-workspace-up" },
	{ key = "Mod+Shift+U", action = "move-workspace-down" },
	{ key = "Mod+Shift+I", action = "move-workspace-up" },

	-- ====================
	-- WINDOW SIZING
	-- ====================

	-- Cycle preset column widths
	{ key = "Mod+R", action = "switch-preset-column-width" },
	-- { key = "Mod+R", action = "switch-preset-column-width-back" },  -- Reverse

	-- Cycle preset window heights
	{ key = "Mod+Shift+R", action = "switch-preset-window-height" },

	-- Reset window height to automatic
	{ key = "Mod+Ctrl+R", action = "reset-window-height" },

	-- Fine column width adjustments
	-- Supports: pixels ("1000", "+5", "-10"), percentages ("25%", "+10%", "-10%")
	{ key = "Mod+Minus", action = "set-column-width", args = { "-10%" } },
	{ key = "Mod+Equal", action = "set-column-width", args = { "+10%" } },

	-- Fine window height adjustments (in columns with multiple windows)
	{ key = "Mod+Shift+Minus", action = "set-window-height", args = { "-10%" } },
	{ key = "Mod+Shift+Equal", action = "set-window-height", args = { "+10%" } },

	-- ====================
	-- MOUSE WHEEL BINDINGS
	-- ====================

	-- Workspace switching
	{ key = "Mod+WheelScrollDown", action = "focus-workspace-down", cooldown_ms = 150 },
	{ key = "Mod+WheelScrollUp", action = "focus-workspace-up", cooldown_ms = 150 },
	{ key = "Mod+Ctrl+WheelScrollDown", action = "move-column-to-workspace-down", cooldown_ms = 150 },
	{ key = "Mod+Ctrl+WheelScrollUp", action = "move-column-to-workspace-up", cooldown_ms = 150 },

	-- Column navigation
	{ key = "Mod+WheelScrollRight", action = "focus-column-right" },
	{ key = "Mod+WheelScrollLeft", action = "focus-column-left" },
	{ key = "Mod+Ctrl+WheelScrollRight", action = "move-column-right" },
	{ key = "Mod+Ctrl+WheelScrollLeft", action = "move-column-left" },

	-- Shift + wheel for horizontal scrolling
	{ key = "Mod+Shift+WheelScrollDown", action = "focus-column-right" },
	{ key = "Mod+Shift+WheelScrollUp", action = "focus-column-left" },
	{ key = "Mod+Ctrl+Shift+WheelScrollDown", action = "move-column-right" },
	{ key = "Mod+Ctrl+Shift+WheelScrollUp", action = "move-column-left" },

	-- ====================
	-- OVERVIEW & SCREENSHOTS
	-- ====================

	-- Toggle overview
	{ key = "Mod+O", action = "toggle-overview" },
	-- { key = "Mod+O", action = "open-overview" },
	-- { key = "Mod+O", action = "close-overview" },

	-- Screenshots
	{ key = "Print", action = "screenshot" },
	{ key = "Ctrl+Print", action = "screenshot-screen" },
	{ key = "Alt+Print", action = "screenshot-window" },
}

-- ============================================================================
-- WINDOW RULES
-- ============================================================================
-- Apply special behavior to specific windows based on app-id or title.

local window_rules = {
	-- Fix WezTerm's initial configure bug
	{
		match = { app_id = "^org%.wezfurlong%.wezterm$" },
		default_column_width = {},
	},

	-- Firefox picture-in-picture as floating
	{
		match = {
			app_id = "firefox$",
			title = "^Picture-in-Picture$",
		},
		open_floating = true,
	},

	-- Example: Set window opacity
	-- {
	--     match = { app_id = "Alacritty" },
	--     opacity = 0.9,
	-- },

	-- Example: Block from screen capture
	-- {
	--     match = { app_id = "^org%.keepassxc%.KeePassXC$" },
	--     block_out_from = "screen-capture",  -- or "screencast"
	-- },

	-- Example: Rounded corners (requires prefer-no-csd)
	-- {
	--     match = {},  -- All windows
	--     geometry_corner_radius = 12,
	--     clip_to_geometry = true,
	-- },

	-- Other available properties:
	-- - min_width, max_width, min_height, max_height
	-- - draw_border_with_background
	-- - open_on_output = "HDMI-A-1"
	-- - open_on_workspace = 2
	-- - open_maximized = true
	-- - open_fullscreen = true
}

-- ============================================================================
-- LAYER RULES
-- ============================================================================
-- Rules for layer-shell surfaces (panels, notifications, etc.)

local layer_rules = {
	-- Example: Block notification daemon from screencasts
	-- {
	--     match = {
	--         namespace = "swaync",
	--         at_startup = true,
	--     },
	--     block_out_from = "screencast",
	-- },
}

-- ============================================================================
-- STARTUP COMMANDS
-- ============================================================================
-- Programs to launch when niri starts.
-- Note: When running as a session, xdg-desktop-autostart also works.

local spawn_at_startup = {
	"waybar",
	"swaync",
	"kitty",
}

-- Shell commands with pipes, variables, etc.
local spawn_sh_at_startup = {
	-- "dbus-update-activation-environment --systemd WAYLAND_DISPLAY XDG_CURRENT_DESKTOP",
}

-- ============================================================================
-- ENVIRONMENT VARIABLES
-- ============================================================================
-- Set environment variables for child processes.

local environment = {
	-- Example: Set default apps
	-- BROWSER = "firefox",
	-- TERMINAL = "alacritty",

	-- Example: Enable Wayland for Qt apps
	-- QT_QPA_PLATFORM = "wayland",

	-- Example: GBM backend for SDL
	-- SDL_VIDEODRIVER = "wayland",
}

-- ============================================================================
-- PREFER NO CSD
-- ============================================================================
-- Ask clients to omit client-side decorations.
-- After changing this, restart apps for it to take effect.

local prefer_no_csd = false

-- ============================================================================
-- WORKSPACES
-- ============================================================================
-- Pre-configure workspaces with names.

local workspaces = {
	-- { name = "1" },
	-- { name = "2" },
	-- { name = "browser" },
	-- { name = "code" },
}

-- ============================================================================
-- APPLY CONFIGURATION
-- ============================================================================
-- Apply the configuration using niri.apply_config()

-- Why use niri.apply_config() instead of return?
-- 1. Allows you to do additional scripting AFTER config is applied
-- 2. More explicit and clear about what's happening
-- 3. Enables sequential execution - no need for async callbacks
-- 4. You can still use local variables and helper functions

-- The old pattern (return { ... }) still works for backward compatibility,
-- but niri.apply_config() is the recommended approach.

niri.apply_config({
	-- Core configuration
	input = input,
	layout = layout,
	cursor = cursor,

	-- Output configuration (monitor-specific)
	-- outputs = outputs,

	-- Features
	gestures = gestures,
	recent_windows = recent_windows,
	overview = overview,
	animations = animations,

	-- UI configuration
	clipboard = clipboard,
	hotkey_overlay = hotkey_overlay,
	config_notification = config_notification,

	-- Advanced
	debug = debug,
	xwayland_satellite = xwayland_satellite,

	-- Miscellaneous
	screenshot = screenshot,
	environment = environment,
	prefer_no_csd = prefer_no_csd,

	-- Bindings and rules
	binds = binds,
	window_rules = window_rules,
	-- layer_rules = layer_rules,

	-- Startup
	spawn_at_startup = spawn_at_startup,
	-- spawn_sh_at_startup = spawn_sh_at_startup,

	-- Workspaces
	-- workspaces = workspaces,
})

-- ============================================================================
-- POST-CONFIG SCRIPTING
-- ============================================================================
-- Everything below this line runs AFTER the configuration has been applied.
-- This is the perfect place to add:

-- 1. Event listeners (when event system is implemented)
--    Example:
-- niri.on("window_opened", function(window)
-- 	niri.log("New window: " .. window.title)
-- end)

-- 2. Runtime state queries
--    Example:
--    local windows = niri.runtime.windows()
--    niri.log("Current window count: " .. #windows)

-- 3. Dynamic configuration based on runtime state
--    Example:
--    local outputs = niri.runtime.outputs()
--    if #outputs > 1 then
--        niri.log("Multiple monitors detected!")
--    end

-- 4. Custom initialization logic
--    Example:
--    local hostname = os.getenv("HOSTNAME")
--    if hostname == "work-laptop" then
--        -- Spawn work-specific apps
--    end

-- 5. Debugging and diagnostics
--    Example:
--    niri.log("Config loaded with " .. #binds .. " keybindings")

-- Simple example: Log that configuration was loaded successfully
niri.log("Niri configuration loaded successfully!")
niri.log("Loaded " .. #binds .. " keybindings and " .. #spawn_at_startup .. " startup command(s)")
