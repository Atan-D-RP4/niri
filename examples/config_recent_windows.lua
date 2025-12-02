-- Recent Windows Configuration Example
-- This demonstrates the recent_windows configuration added in the latest Niri version.
-- Recent Windows shows previews of your most recently used windows.

if niri then
	niri.log("Loading recent windows configuration example...")
end

-- ============================================================================
-- RECENT WINDOWS CONFIGURATION
-- ============================================================================
-- Recent Windows provides quick access to your most recently used windows
-- with visual previews. This is useful for quickly switching between frequently
-- used applications.

local recent_windows = {
	-- Enable or disable recent windows tracking
	on = true,

	-- Delay before showing recent windows (in milliseconds)
	-- This allows you to perform the gesture without triggering the UI
	open_delay_ms = 500,

	-- Highlight configuration for active/urgent workspaces
	highlight = {
		-- Color of the highlight for the active workspace
		active_color = "#7fc8ff",

		-- Color of the highlight for urgent workspaces
		urgent_color = "#ff6b6b",

		-- Padding around the highlight
		padding = 8,

		-- Corner radius of the highlight box
		corner_radius = 10,
	},

	-- Preview configuration for window thumbnails
	previews = {
		-- Maximum height of window preview thumbnails
		max_height = 200,

		-- Maximum scale factor for window previews
		max_scale = 0.5,
	},
}

-- ============================================================================
-- EXAMPLE: COMBINED WITH OTHER ANIMATIONS
-- ============================================================================
-- The recent_windows_close animation can be configured alongside other animations

local animations = {
	-- Window open/close animations
	window_open = {
		duration_ms = 150,
		curve = "ease-out-expo",
	},
	window_close = {
		duration_ms = 150,
		curve = "ease-out-expo",
	},

	-- Recent windows animation when closing
	recent_windows_close = {
		duration_ms = 200,
		curve = "ease-out-cubic",
	},

	-- Workspace switch animation
	workspace_switch = {
		spring = {
			damping_ratio = 0.8,
			stiffness = 4,
		},
	},
}

-- ============================================================================
-- EXAMPLE: READING RECENT WINDOWS CONFIG FROM RUNTIME
-- ============================================================================
-- You can read the recent_windows configuration from the Lua runtime API

if niri then
	-- Check if recent windows is enabled (off = false means it's enabled)
	if not niri.config.recent_windows.off then
		niri.log("Recent windows is enabled")
		niri.log(string.format("Open delay: %dms", niri.config.recent_windows.open_delay_ms))
		niri.log(string.format("Preview max height: %d", niri.config.recent_windows.previews.max_height))
	else
		niri.log("Recent windows is disabled")
	end
end

-- To use this configuration, uncomment the sections below and add them to your
-- main niri.lua configuration file:
-- recent_windows = recent_windows
-- animations = animations
