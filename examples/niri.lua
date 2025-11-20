-- Niri Lua Configuration Example
-- Place this file at ~/.config/niri/config.lua
-- See the wiki for detailed documentation: https://yalter.github.io/niri/Configuration:-Introduction

-- Log only if running within Niri (niri global is available)
if niri then
	niri.log("Loading Niri Lua configuration example...")
end

-- ============================================================================
-- INPUT CONFIGURATION
-- ============================================================================
-- Input device configuration
-- Full documentation: https://yalter.github.io/niri/Configuration:-Input

local input = {
	keyboard = {
		-- XKB configuration for keyboard layout
		-- Uncomment and modify to set custom keyboard layout:
		xkb = {
			layout = "us",
			variant = "intl,phonetic",
			-- options = "grp:win_space_toggle,compose:ralt,ctrl:nocaps",
		},
		-- Enable numlock on startup (omit to disable)
		numlock = false,
	},

	-- Next sections include libinput settings.
	-- Omitting settings disables them, or leaves them at their default values.
	touchpad = {
		-- off = true,
		tap = true,
		-- dwt = true,
		-- dwtp = true,
		-- drag = false,
		-- drag_lock = true,
		natural_scroll = true,
		-- accel_speed = 0.2,
		-- accel_profile = "flat",
		-- scroll_method = "two-finger",
		-- disabled_on_external_mouse = true,
	},

	mouse = {
		-- off = true,
		-- natural_scroll = false,
		-- accel_speed = 0.2,
		-- accel_profile = "flat",
		-- scroll_method = "no-scroll",
	},

	trackpoint = {
		-- off = true,
		-- natural_scroll = false,
		-- accel_speed = 0.2,
		-- accel_profile = "flat",
		-- scroll_method = "on-button-down",
		-- scroll_button = 273,
		-- scroll_button_lock = true,
		-- middle_emulation = true,
	},

	-- Uncomment to make the mouse warp to the center of newly focused windows.
	-- warp_mouse_to_focus = true,

	-- Focus windows and outputs automatically when moving the mouse into them.
	-- Setting max_scroll_amount="0%" makes it work only on windows already fully on screen.
	-- focus_follows_mouse = { max_scroll_amount = "0%" },
}

-- ============================================================================
-- OUTPUT CONFIGURATION
-- ============================================================================
-- You can configure outputs by their name, which you can find
-- by running `niri msg outputs` while inside a niri instance.
-- Full documentation: https://yalter.github.io/niri/Configuration:-Outputs

-- Uncomment and modify to configure displays:
--[[
local outputs = {
    {
        name = "eDP-1",  -- laptop internal display
        -- Uncomment to disable this output.
        -- off = true,

        -- Resolution and, optionally, refresh rate of the output.
        -- The format is "<width>x<height>" or "<width>x<height>@<refresh rate>".
        -- If the refresh rate is omitted, niri will pick the highest refresh rate
        -- for the resolution. If invalid, niri will pick one automatically.
        mode = "1920x1080@120.030",

        -- You can use integer or fractional scale, for example use 1.5 for 150% scale.
        scale = 2.0,

        -- Transform allows to rotate the output counter-clockwise
        -- Valid values: normal, 90, 180, 270, flipped, flipped-90, flipped-180, flipped-270
        transform = "normal",

        -- Position of the output in the global coordinate space.
        -- This affects directional monitor actions like "focus-monitor-left", and cursor movement.
        -- Output scale and rotation has to be taken into account for positioning:
        -- outputs are sized in logical, or scaled, pixels.
        position = { x = 1280, y = 0 },
    },
}
--]]

-- ============================================================================
-- LAYOUT CONFIGURATION
-- ============================================================================
-- Settings that influence how windows are positioned and sized.
-- Full documentation: https://yalter.github.io/niri/Configuration:-Layout

local layout = {
	-- Set gaps around windows in logical pixels.
	gaps = 16,

	backgroud_color = "transparent",

	-- When to center a column when changing focus
	-- Options: "never" (default), "always", "on-overflow"
	center_focused_column = "never",

	-- Customize the widths that "switch-preset-column-width" (Mod+R) toggles between.
	-- Proportion sets the width as a fraction of the output width, taking gaps into account.
	-- For example, you can perfectly fit four windows sized "proportion 0.25" on an output.
	-- The default preset widths are 1/3, 1/2 and 2/3 of the output.
	preset_column_widths = {
		{ proportion = 0.33333 }, -- 1/3
		{ proportion = 0.5 }, -- 1/2
		{ proportion = 0.66667 }, -- 2/3
		-- You can also use fixed widths:
		-- { fixed = 1920 },
	},

	-- You can also customize the heights that "switch-preset-window-height" (Mod+Shift+R) toggles between.
	-- preset_window_heights = { ... },

	-- Change the default width of new windows.
	default_column_width = { proportion = 0.5 },
	-- If you leave it empty, the windows themselves will decide their initial width.
	-- default_column_width = {},

	-- By default focus ring and border are rendered as a solid background rectangle
	-- behind windows. That is, they will show up through semitransparent windows.
	-- This is because windows using client-side decorations can have an arbitrary shape.

	-- If you don't like that, uncomment `prefer_no_csd` below.
	-- Niri will draw focus ring and border *around* windows that agree to omit their
	-- client-side decorations.

	-- Alternatively, you can override it with a window rule called
	-- `draw_border_with_background`.
	-- prefer_no_csd = true,

	-- You can change how the focus ring looks.
	focus_ring = {
		-- Uncomment this line to disable the focus ring.
		-- off = true,

		-- How many logical pixels the ring extends out from the windows.
		width = 4,

		-- Colors can be set in a variety of ways:
		-- - CSS named colors: "red"
		-- - RGB hex: "#rgb", "#rgba", "#rrggbb", "#rrggbbaa"
		-- - CSS-like notation: "rgb(255, 127, 0)", rgba(), hsl() and a few others.

		-- Color of the ring on the active monitor.
		active_color = "#7fc8ff",

		-- Color of the ring on inactive monitors.
		-- The focus ring only draws around the active window, so the only place
		-- where you can see its inactive_color is on other monitors.
		inactive_color = "#505050",

		-- You can also use gradients. They take precedence over solid colors.
		-- Gradients are rendered the same as CSS linear-gradient(angle, from, to).
		-- The angle is the same as in linear-gradient, and is optional,
		-- defaulting to 180 (top-to-bottom gradient).
		-- You can use any CSS linear-gradient tool on the web to set these up.
		-- Changing the color space is also supported, check the wiki for more info.

		-- active_gradient = { from = "#80c8ff", to = "#c7ff7f", angle = 45 },

		-- You can also color the gradient relative to the entire view
		-- of the workspace, rather than relative to just the window itself.
		-- To do that, set relative_to="workspace-view".

		-- inactive_gradient = { from = "#505050", to = "#808080", angle = 45, relative_to = "workspace-view" },
	},

	-- You can also add a border. It's similar to the focus ring, but always visible.
	border = {
		-- The settings are the same as for the focus ring.
		-- If you enable the border, you probably want to disable the focus ring.
		off = true,

		width = 4,
		active_color = "#ffc87f",
		inactive_color = "#505050",

		-- Color of the border around windows that request your attention.
		urgent_color = "#9b0000",

		-- Gradients can use a few different interpolation color spaces.
		-- For example, this is a pastel rainbow gradient via in="oklch longer hue".

		-- active_gradient = { from = "#e5989b", to = "#ffb4a2", angle = 45, relative_to = "workspace-view", in_ = "oklch longer hue" },

		-- inactive_gradient = { from = "#505050", to = "#808080", angle = 45, relative_to = "workspace-view" },
	},

	-- You can enable drop shadows for windows.
	shadow = {
		-- Uncomment the next line to enable shadows.
		-- on = true,

		-- By default, the shadow draws only around its window, and not behind it.
		-- Uncomment this setting to make the shadow draw behind its window.

		-- Note that niri has no way of knowing about the CSD window corner
		-- radius. It has to assume that windows have square corners, leading to
		-- shadow artifacts inside the CSD rounded corners. This setting fixes
		-- those artifacts.

		-- However, instead you may want to set prefer_no_csd and/or
		-- geometry_corner_radius. Then, niri will know the corner radius and
		-- draw the shadow correctly, without having to draw it behind the
		-- window. These will also remove client-side shadows if the window
		-- draws any.

		-- draw_behind_window = true,

		-- You can change how shadows look. The values below are in logical
		-- pixels and match the CSS box-shadow properties.

		-- Softness controls the shadow blur radius.
		softness = 30,

		-- Spread expands the shadow.
		spread = 5,

		-- Offset moves the shadow relative to the window.
		offset = { x = 0, y = 5 },

		-- You can also change the shadow color and opacity.
		color = "#0007",
	},

	-- Struts shrink the area occupied by windows, similarly to layer-shell panels.
	-- You can think of them as a kind of outer gaps. They are set in logical pixels.
	-- Left and right struts will cause the next window to the side to always be visible.
	-- Top and bottom struts will simply add outer gaps in addition to the area occupied by
	-- layer-shell panels and regular gaps.
	struts = {
		-- left = 64,
		-- right = 64,
		-- top = 64,
		-- bottom = 64,
	},
}

-- ============================================================================
-- STARTUP COMMANDS
-- ============================================================================
-- Add lines like this to spawn processes at startup.
-- Note that running niri as a session supports xdg-desktop-autostart,
-- which may be more convenient to use.

local startup_commands = {
	-- This line starts waybar, a commonly used bar for Wayland compositors.
	"waybar",
	"swaync",
	"kitty",

	-- To run a shell command (with variables, pipes, etc.), use spawn-sh:
	-- "sh -c 'qs -c ~/source/qs/MyAwesomeShell'",
}

-- ============================================================================
-- HOTKEY OVERLAY
-- ============================================================================

local hotkey_overlay = {
	-- Uncomment this line to disable the "Important Hotkeys" pop-up at startup.
	-- skip_at_startup = true,
}

-- ============================================================================
-- CLIENT-SIDE DECORATION PREFERENCES
-- ============================================================================
-- Uncomment this line to ask the clients to omit their client-side decorations if possible.
-- If the client will specifically ask for CSD, the request will be honored.
-- Additionally, clients will be informed that they are tiled, removing some client-side rounded corners.
-- This option will also fix border/focus ring drawing behind some semitransparent windows.
-- After enabling or disabling this, you need to restart the apps for this to take effect.

-- prefer_no_csd = true,

-- ============================================================================
-- SCREENSHOT CONFIGURATION
-- ============================================================================
-- You can change the path where screenshots are saved.
-- A ~ at the front will be expanded to the home directory.
-- The path is formatted with strftime(3) to give you the screenshot date and time.

local screenshot = {
	path = "~/Media/images/Screenshots/Screenshot_from_%Y-%m-%d %H-%M-%S.png",
	-- You can also set this to null to disable saving screenshots to disk.
	-- path = nil,
}

-- ============================================================================
-- ANIMATION CONFIGURATION
-- ============================================================================
-- Animation settings and documentation:
-- https://yalter.github.io/niri/Configuration:-Animations

local animations = {
	-- Uncomment to turn off all animations.
	-- off = true,

	-- Slow down all animations by this factor. Values below 1 speed them up instead.
	-- slowdown = 3.0,
}

-- ============================================================================
-- WINDOW RULES
-- ============================================================================
-- Window rules let you adjust behavior for individual windows.
-- Full documentation: https://yalter.github.io/niri/Configuration:-Window-Rules

local window_rules = {
	-- Work around WezTerm's initial configure bug
	-- by setting an empty default_column_width.
	{
		-- This regular expression is intentionally made as specific as possible,
		-- since this is the default config, and we want no false positives.
		-- You can get away with just app_id="wezterm" if you want.
		match = { app_id = "^org%.wezfurlong%.wezterm$" },
		default_column_width = {},
	},

	-- Open the Firefox picture-in-picture player as floating by default.
	{
		-- This app_id regular expression will work for both:
		-- - host Firefox (app_id is "firefox")
		-- - Flatpak Firefox (app_id is "org.mozilla.firefox")
		match = { app_id = "firefox$", title = "^Picture%-in%-Picture$" },
		open_floating = true,
	},

	-- Example: block out two password managers from screen capture.
	-- (This example rule is commented out by default.)
	--[[
    {
        match = { app_id = "^org%.keepassxc%.KeePassXC$" },
        block_out_from = "screen-capture",
        -- Use this instead if you want them visible on third-party screenshot tools.
        -- block_out_from = "screencast",
    },
    {
        match = { app_id = "^org%.gnome%.World%.Secrets$" },
        block_out_from = "screen-capture",
    },
    --]]

	-- Example: enable rounded corners for all windows.
	-- (This example rule is commented out by default.)
	--[[
    {
        match = { app_id = ".*" },
        geometry_corner_radius = 12,
        clip_to_geometry = true,
    },
    --]]
}

local layer_rules = {
	match = { namespace = "swaync", at_startup = true },
	block_out_from = "screencast",
	block_out_from = "screencapture",
}

local recent_windows = {
	highlight = {
		active_color = "#00000000",
		urgent_color = "#ff9999ff",
		padding = 30,
		corner_radius = 2,
	},
}

-- ============================================================================
-- KEYBINDINGS
-- ============================================================================
-- Keys consist of modifiers separated by + signs, followed by an XKB key name
-- in the end. To find an XKB name for a particular key, you may use a program
-- like wev.

-- "Mod" is a special modifier equal to Super when running on a TTY, and to Alt
-- when running as a winit window.

-- Most actions that you can bind here can also be invoked programmatically with
-- `niri msg action do-something`.

local binds = {
	-- Show hotkey overlay
	-- Mod-Shift-/, which is usually the same as Mod-?,
	-- shows a list of important hotkeys.
	{ key = "Mod+Slash", action = "show-hotkey-overlay" },

	-- Suggested binds for running programs: terminal, app launcher, screen locker.
	{
		key = "Mod+T",
		action = "spawn",
		args = { "kitty" },
		title = "Open a Terminal: kitty",
	},
	{
		key = "Mod+D",
		action = "spawn-sh",
		args = { "hyde-shell rofilaunch.sh" },
		title = "Run an Application: HyDE Rofi",
	},
	{
		key = "Super+Alt+L",
		action = "spawn-sh",
		args = { "hyde-shell hyprlock.sh" },
		title = "Lock the Screen: HyDE Hyprlock",
	},

	-- Use spawn-sh to run a shell command. Do this if you need pipes, multiple commands, etc.
	-- Note: the entire command goes as a single argument. It's passed verbatim to `sh -c`.
	-- For example, this is a standard bind to toggle the screen reader (orca).
	{
		key = "Super+Alt+S",
		action = "spawn-sh",
		args = { "pkill orca || exec orca" },
		allow_when_locked = true,
		title = nil, -- Set title to nil to disable in hotkey overlay
	},

	-- Example volume keys mappings for PipeWire & WirePlumber.
	-- The allow_when_locked=true property makes them work even when the session is locked.
	-- Using spawn-sh allows to pass multiple arguments together with the command.
	{
		key = "XF86AudioRaiseVolume",
		action = "spawn-sh",
		args = { "wpctl set-volume @DEFAULT_AUDIO_SINK@ 0.1+" },
		allow_when_locked = true,
	},
	{
		key = "XF86AudioLowerVolume",
		action = "spawn-sh",
		args = { "wpctl set-volume @DEFAULT_AUDIO_SINK@ 0.1-" },
		allow_when_locked = true,
	},
	{
		key = "XF86AudioMute",
		action = "spawn-sh",
		args = { "wpctl set-mute @DEFAULT_AUDIO_SINK@ toggle" },
		allow_when_locked = true,
	},
	{
		key = "XF86AudioMicMute",
		action = "spawn-sh",
		args = { "wpctl set-mute @DEFAULT_AUDIO_SOURCE@ toggle" },
		allow_when_locked = true,
	},

	-- Example media keys mapping using playerctl.
	-- This will work with any MPRIS-enabled media player.
	{
		key = "XF86AudioPlay",
		action = "spawn-sh",
		args = { "playerctl play-pause" },
		allow_when_locked = true,
	},
	{
		key = "XF86AudioStop",
		action = "spawn-sh",
		args = { "playerctl stop" },
		allow_when_locked = true,
	},
	{
		key = "XF86AudioPrev",
		action = "spawn-sh",
		args = { "playerctl previous" },
		allow_when_locked = true,
	},
	{
		key = "XF86AudioNext",
		action = "spawn-sh",
		args = { "playerctl next" },
		allow_when_locked = true,
	},

	-- Example brightness key mappings for brightnessctl.
	-- You can use regular spawn with multiple arguments too (to avoid going through "sh"),
	-- but you need to manually put each argument in separate "" quotes.
	{
		key = "XF86MonBrightnessUp",
		action = "spawn",
		args = { "brightnessctl", "--class=backlight", "set", "+10%" },
		allow_when_locked = true,
	},
	{
		key = "XF86MonBrightnessDown",
		action = "spawn",
		args = { "brightnessctl", "--class=backlight", "set", "10%-" },
		allow_when_locked = true,
	},

	-- Open/close the Overview: a zoomed-out view of workspaces and windows.
	-- You can also move the mouse into the top-left hot corner,
	-- or do a four-finger swipe up on a touchpad.
	{ key = "Mod+O", action = "toggle-overview", repeat_key = false },

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

	-- Alternative commands that move across workspaces when reaching
	-- the first or last window in a column.
	-- { key = "Mod+J", action = "focus-window-or-workspace-down" },
	-- { key = "Mod+K", action = "focus-window-or-workspace-up" },
	-- { key = "Mod+Ctrl+J", action = "move-window-down-or-to-workspace-down" },
	-- { key = "Mod+Ctrl+K", action = "move-window-up-or-to-workspace-up" },

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

	-- Alternatively, there are commands to move just a single window:
	-- { key = "Mod+Shift+Ctrl+Left", action = "move-window-to-monitor-left" },
	-- ...

	-- And you can also move a whole workspace to another monitor:
	-- { key = "Mod+Shift+Ctrl+Left", action = "move-workspace-to-monitor-left" },
	-- ...

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

	-- Alternatively, there are commands to move just a single window:
	-- { key = "Mod+Ctrl+Page_Down", action = "move-window-to-workspace-down" },
	-- ...

	-- Move workspace
	{ key = "Mod+Shift+Page_Down", action = "move-workspace-down" },
	{ key = "Mod+Shift+Page_Up", action = "move-workspace-up" },
	{ key = "Mod+Shift+U", action = "move-workspace-down" },
	{ key = "Mod+Shift+I", action = "move-workspace-up" },

	-- You can bind mouse wheel scroll ticks using the following syntax.
	-- These binds will change direction based on the natural-scroll setting.

	-- To avoid scrolling through workspaces really fast, you can use
	-- the cooldown_ms property. The bind will be rate-limited to this value.
	-- You can set a cooldown on any bind, but it's most useful for the wheel.
	{ key = "Mod+WheelScrollDown", action = "focus-workspace-down", cooldown = 150 },
	{ key = "Mod+WheelScrollUp", action = "focus-workspace-up", cooldown = 150 },
	{ key = "Mod+Ctrl+WheelScrollDown", action = "move-column-to-workspace-down", cooldown = 150 },
	{ key = "Mod+Ctrl+WheelScrollUp", action = "move-column-to-workspace-up", cooldown = 150 },

	{ key = "Mod+WheelScrollRight", action = "focus-column-right" },
	{ key = "Mod+WheelScrollLeft", action = "focus-column-left" },
	{ key = "Mod+Ctrl+WheelScrollRight", action = "move-column-right" },
	{ key = "Mod+Ctrl+WheelScrollLeft", action = "move-column-left" },

	-- Usually scrolling up and down with Shift in applications results in
	-- horizontal scrolling; these binds replicate that.
	{ key = "Mod+Shift+WheelScrollDown", action = "focus-column-right" },
	{ key = "Mod+Shift+WheelScrollUp", action = "focus-column-left" },
	{ key = "Mod+Ctrl+Shift+WheelScrollDown", action = "move-column-right" },
	{ key = "Mod+Ctrl+Shift+WheelScrollUp", action = "move-column-left" },

	-- Similarly, you can bind touchpad scroll "ticks".
	-- Touchpad scrolling is continuous, so for these binds it is split into
	-- discrete intervals.
	-- These binds are also affected by touchpad's natural-scroll, so these
	-- example binds are "inverted", since we have natural-scroll enabled for
	-- touchpads by default.
	-- { key = "Mod+TouchpadScrollDown", action = "spawn-sh", args = { "wpctl set-volume @DEFAULT_AUDIO_SINK@ 0.02+" } },
	-- { key = "Mod+TouchpadScrollUp", action = "spawn-sh", args = { "wpctl set-volume @DEFAULT_AUDIO_SINK@ 0.02-" } },

	-- You can refer to workspaces by index. However, keep in mind that
	-- niri is a dynamic workspace system, so these commands are kind of
	-- "best effort". Trying to refer to a workspace index bigger than
	-- the current workspace count will instead refer to the bottommost
	-- (empty) workspace.

	-- For example, with 2 workspaces + 1 empty, indices 3, 4, 5 and so on
	-- will all refer to the 3rd workspace.
	{ key = "Mod+1", action = "focus-workspace", args = { 1 } },
	{ key = "Mod+2", action = "focus-workspace", args = { 2 } },
	{ key = "Mod+3", action = "focus-workspace", args = { 3 } },
	{ key = "Mod+4", action = "focus-workspace", args = { 4 } },
	{ key = "Mod+5", action = "focus-workspace", args = { 5 } },
	{ key = "Mod+6", action = "focus-workspace", args = { 6 } },
	{ key = "Mod+7", action = "focus-workspace", args = { 7 } },
	{ key = "Mod+8", action = "focus-workspace", args = { 8 } },
	{ key = "Mod+9", action = "focus-workspace", args = { 9 } },
	{ key = "Mod+Ctrl+1", action = "move-column-to-workspace", args = { 1 } },
	{ key = "Mod+Ctrl+2", action = "move-column-to-workspace", args = { 2 } },
	{ key = "Mod+Ctrl+3", action = "move-column-to-workspace", args = { 3 } },
	{ key = "Mod+Ctrl+4", action = "move-column-to-workspace", args = { 4 } },
	{ key = "Mod+Ctrl+5", action = "move-column-to-workspace", args = { 5 } },
	{ key = "Mod+Ctrl+6", action = "move-column-to-workspace", args = { 6 } },
	{ key = "Mod+Ctrl+7", action = "move-column-to-workspace", args = { 7 } },
	{ key = "Mod+Ctrl+8", action = "move-column-to-workspace", args = { 8 } },
	{ key = "Mod+Ctrl+9", action = "move-column-to-workspace", args = { 9 } },

	-- Alternatively, there are commands to move just a single window:
	-- { key = "Mod+Ctrl+1", action = "move-window-to-workspace", args = { 1 } },

	-- Switches focus between the current and the previous workspace.
	-- { key = "Mod+Tab", action = "focus-workspace-previous" },

	-- The following binds move the focused window in and out of a column.
	-- If the window is alone, they will consume it into the nearby column to the side.
	-- If the window is already in a column, they will expel it out.
	{ key = "Mod+BracketLeft", action = "consume-or-expel-window-left" },
	{ key = "Mod+BracketRight", action = "consume-or-expel-window-right" },

	-- Consume one window from the right to the bottom of the focused column.
	{ key = "Mod+Comma", action = "consume-window-into-column" },
	-- Expel the bottom window from the focused column to the right.
	{ key = "Mod+Period", action = "expel-window-from-column" },

	-- Column width management
	{ key = "Mod+R", action = "switch-preset-column-width" },
	-- Cycling through the presets in reverse order is also possible.
	-- { key = "Mod+R", action = "switch-preset-column-width-back" },
	{ key = "Mod+Shift+R", action = "switch-preset-window-height" },
	{ key = "Mod+Ctrl+R", action = "reset-window-height" },

	-- Column and window sizing
	{ key = "Mod+F", action = "maximize-column" },
	{ key = "Mod+Shift+F", action = "fullscreen-window", repeat_key = false },

	-- Expand the focused column to space not taken up by other fully visible columns.
	-- Makes the column "fill the rest of the space".
	{ key = "Mod+Ctrl+F", action = "expand-column-to-available-width" },

	-- Column centering
	{ key = "Mod+C", action = "center-column" },

	-- Center all fully visible columns on screen.
	{ key = "Mod+Ctrl+C", action = "center-visible-columns" },

	-- Finer width adjustments.
	-- This command can also:
	-- * set width in pixels: "1000"
	-- * adjust width in pixels: "-5" or "+5"
	-- * set width as a percentage of screen width: "25%"
	-- * adjust width as a percentage of screen width: "-10%" or "+10%"
	-- Pixel sizes use logical, or scaled, pixels. I.e. on an output with scale 2.0,
	-- set-column-width "100" will make the column occupy 200 physical screen pixels.
	{ key = "Mod+Minus", action = "set-column-width", args = { "-10%" } },
	{ key = "Mod+Equal", action = "set-column-width", args = { "+10%" } },

	-- Finer height adjustments when in column with other windows.
	{ key = "Mod+Shift+Minus", action = "set-window-height", args = { "-10%" } },
	{ key = "Mod+Shift+Equal", action = "set-window-height", args = { "+10%" } },

	-- Move the focused window between the floating and the tiling layout.
	{ key = "Mod+V", action = "toggle-window-floating" },
	{ key = "Mod+Shift+V", action = "switch-focus-between-floating-and-tiling" },

	-- Toggle tabbed column display mode.
	-- Windows in this column will appear as vertical tabs,
	-- rather than stacked on top of each other.
	{ key = "Mod+W", action = "toggle-column-tabbed-display" },

	-- Actions to switch layouts.
	-- Note: if you uncomment these, make sure you do NOT have
	-- a matching layout switch hotkey configured in xkb options above.
	-- Having both at once on the same hotkey will break the switching,
	-- since it will switch twice upon pressing the hotkey (once by xkb, once by niri).
	-- { key = "Mod+Space", action = "switch-layout", args = { "next" } },
	-- { key = "Mod+Shift+Space", action = "switch-layout", args = { "prev" } },

	-- Screenshots
	{ key = "Print", action = "screenshot" },
	{ key = "Ctrl+Print", action = "screenshot-screen" },
	{ key = "Alt+Print", action = "screenshot-window" },

	-- Applications such as remote-desktop clients and software KVM switches may
	-- request that niri stops processing the keyboard shortcuts defined here
	-- so they may, for example, forward the key presses as-is to a remote machine.
	-- It's a good idea to bind an escape hatch to toggle the inhibitor,
	-- so a buggy application can't hold your session hostage.

	-- The allow_inhibiting=false property can be applied to other binds as well,
	-- which ensures niri always processes them, even when an inhibitor is active.
	{ key = "Mod+Escape", action = "toggle-keyboard-shortcuts-inhibit", allow_inhibiting = false },

	-- The quit action will show a confirmation dialog to avoid accidental exits.
	{ key = "Mod+Shift+E", action = "quit" },
	{ key = "Ctrl+Alt+Delete", action = "quit" },

	-- Powers off the monitors. To turn them back on, do any input like
	-- moving the mouse or pressing any other key.
	{ key = "Mod+Shift+P", action = "power-off-monitors" },
}

-- ============================================================================
-- CONFIGURATION APPLICATION
-- ============================================================================

-- Log only if running within Niri (niri global is available)
if niri then
	niri.log("Niri Lua configuration loaded successfully!")
end
return {
	input = input,
	layout = layout,
	animations = animations,
	hotkey_overlay = hotkey_overlay,
	screenshot = screenshot,
	startup_commands = startup_commands,
	window_rules = window_rules,
	layer_rules = layer_rules,
	recent_windows = recent_windows,
	binds = binds,
}
