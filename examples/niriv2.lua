---@diagnostic disable: inject-field
-- ============================================================================
-- Niri Lua Configuration v2 - Unified API
-- ============================================================================
-- This configuration uses the unified API design:

--   niri.config   - Configuration (read/write with explicit apply)
--   niri.state    - Runtime state queries (read-only)
--   niri.action   - Execute compositor actions (method-style with :)
--   niri.events   - Event system (niri.events:on/once/off)
--   niri.utils    - Utilities (log, debug, warn, error)
--   niri.schedule - Defer execution to next event loop iteration
--   niri.loop     - Timers and time functions (new_timer, now)

-- Key features:
-- - Direct property assignment: `niri.config.layout.gaps = 16`
-- - Explicit apply: `niri.config:apply()`
-- - Collection CRUD: `:list()`, `:get()`, `:add()`, `:set()`, `:remove()`
-- - Events use `niri.events:on(event_name, callback)`
-- - Actions use method-style: `niri.action:spawn({"cmd"})`

-- Documentation: See niri-lua/LUA_SPECIFICATION.md
-- ============================================================================

-- ============================================================================
-- INPUT CONFIGURATION
-- ============================================================================

niri.config.input = {
	keyboard = {
		xkb = {
			layout = "us",
			options = "ctrl:nocaps",
		},
		numlock = false,
	},

	mouse = {
		-- accel_speed = 0.0,
		-- accel_profile = "adaptive",
		-- natural_scroll = false,
	},

	touchpad = {
		tap = true,
		natural_scroll = true,
		-- accel_speed = 0.0,
		-- dwt = true,
		-- dwtp = true,
	},

	trackpoint = {
		-- accel_speed = 0.0,
		-- accel_profile = "adaptive",
	},

	touch = {
		-- Enable natural scrolling for touchscreen gestures
		natural_scroll = true,
		-- off = false,
		-- map_to_output = "eDP-1",
	},

	-- warp_mouse_to_focus = false,
	-- focus_follows_mouse = { max_scroll_amount = "0%" },
}

-- ============================================================================
-- LAYOUT CONFIGURATION
-- ============================================================================

niri.config.layout = {
	gaps = 16,
	center_focused_column = "never",

	preset_column_widths = {
		{ proportion = 1.0 / 3.0 },
		{ proportion = 1.0 / 2.0 },
		{ proportion = 2.0 / 3.0 },
	},

	default_column_width = { proportion = 0.5 },

	preset_window_heights = {
		{ proportion = 1.0 / 3.0 },
		{ proportion = 1.0 / 2.0 },
		{ proportion = 2.0 / 3.0 },
	},

	focus_ring = {
		width = 4,
		active_color = "#7fc8ff",
		inactive_color = "#505050",
	},

	border = {
		off = true,
		width = 4,
		active_color = "#ffc87f",
		inactive_color = "#505050",
		urgent_color = "#9b0000",
	},

	shadow = {
		softness = 30,
		spread = 5,
		offset = { x = 0, y = 5 },
		color = "#0007",
	},
}

-- ============================================================================
-- CURSOR CONFIGURATION
-- ============================================================================

niri.config.cursor = {
	xcursor_size = 24,
	hide_when_typing = false,
}

-- ============================================================================
-- GESTURES CONFIGURATION
-- ============================================================================

niri.config.gestures = {
	-- hot_corners = {
	--     off = false,
	--     top_left = "toggle-overview",
	-- },
}

-- ============================================================================
-- RECENT WINDOWS (MRU) CONFIGURATION
-- ============================================================================

niri.config.recent_windows = {
	off = false,
	open_delay_ms = 150,
	highlight = {
		active_color = "#999999",
		urgent_color = "#ff9999",
		padding = 30,
		corner_radius = 5,
	},
	previews = {
		max_height = 480,
		max_scale = 0.5,
	},
}

-- ============================================================================
-- OVERVIEW CONFIGURATION
-- ============================================================================

niri.config.overview = {
	zoom = 0.5,
	backdrop_color = "#ffffaaff",
	workspace_shadow = {
		softness = 30,
		spread = 5,
		offset = { x = 0, y = 5 },
		color = "#0007",
	},
}

-- ============================================================================
-- ANIMATIONS CONFIGURATION
-- ============================================================================

niri.config.animations = {
	-- off = false,
	-- slowdown = 1.0,
}

-- ============================================================================
-- CLIPBOARD CONFIGURATION
-- ============================================================================

niri.config.clipboard = {
	disable_primary = false,
}

-- ============================================================================
-- HOTKEY OVERLAY CONFIGURATION
-- ============================================================================

niri.config.hotkey_overlay = {
	skip_at_startup = true,
}

-- ============================================================================
-- CONFIG NOTIFICATION CONFIGURATION
-- ============================================================================

niri.config.config_notification = {
	disable_failed = false,
}

-- ============================================================================
-- DEBUG CONFIGURATION
-- ============================================================================

niri.config.debug = {
	-- disable_direct_scanout = false,
}

-- ============================================================================
-- XWAYLAND SATELLITE CONFIGURATION
-- ============================================================================

niri.config.xwayland_satellite = {
	off = false or niri.fs.which("xwayland-satellite") == nil,
}

-- ============================================================================
-- SCREENSHOT CONFIGURATION
-- ============================================================================

niri.config.screenshot_path = "~/Pictures/Screenshots/Screenshot from %Y-%m-%d %H-%M-%S.png"

-- ============================================================================
-- PREFER NO Client Side Decorations (CSD)
-- ============================================================================

niri.config.prefer_no_csd = true

-- ============================================================================
-- OUTPUT CONFIGURATION (Collection)
-- ============================================================================
-- Use CRUD operations for collection-type configurations

niri.config.outputs:add({
	name = "eDP-1",
	mode = "1920x1080@60",
	scale = 1.5,
})

-- niri.config.outputs:add({
--     name = "HDMI-A-1",
--     mode = "3840x2160@60",
--     scale = 1.5,
--     position = { x = 1920, y = 0 },
-- })

-- ============================================================================
-- WORKSPACES CONFIGURATION (Collection)
-- ============================================================================

-- niri.config.workspaces:add({ name = "browser" })
-- niri.config.workspaces:add({ name = "code" })

-- ============================================================================
-- ENVIRONMENT VARIABLES (Collection)
-- ============================================================================

-- niri.config.environment:add({ name = "BROWSER", value = "firefox" })
-- niri.config.environment:add({ name = "QT_QPA_PLATFORM", value = "wayland" })

-- ============================================================================
-- KEYBINDINGS (Collection)
-- ============================================================================

-- System & Compositor
niri.config.binds:add({
	{ key = "Mod+Shift+Slash", action = "show-hotkey-overlay" },
	{ key = "Mod+Shift+E", action = "quit" },
	{ key = "Ctrl+Alt+Delete", action = "quit" },
	{ key = "Mod+Shift+P", action = "power-off-monitors" },
	{ key = "Mod+Escape", action = "toggle-keyboard-shortcuts-inhibit" },
})

-- Application Launching
niri.config.binds:add({
	{ key = "Mod+T", action = "spawn", args = { "kitty" } },
	{ key = "Mod+D", action = "spawn-sh", args = { "hyde-shell rofilaunch.sh" } },
	{ key = "Mod+N", action = "spawn-sh", args = { "swaync-client --toggle-panel" } },
	{ key = "Super+Alt+L", action = "spawn", args = { "hyde-shell hyprlock.sh" } },
})

-- Window Management
niri.config.binds:add({
	{ key = "Mod+Q", action = "close-window" },
	{ key = "Mod+Shift+F", action = "fullscreen-window" },
	{ key = "Mod+F", action = "maximize-column" },
	{ key = "Mod+Ctrl+F", action = "expand-column-to-available-width" },
	{ key = "Mod+C", action = "center-column" },
	{ key = "Mod+Ctrl+C", action = "center-visible-columns" },
	{ key = "Mod+V", action = "toggle-window-floating" },
	{ key = "Mod+Shift+V", action = "switch-focus-between-floating-and-tiling" },
	{ key = "Mod+W", action = "toggle-column-tabbed-display" },
})

-- Window Focus
niri.config.binds:add({
	{ key = "Mod+Left", action = "focus-column-left" },
	{ key = "Mod+Right", action = "focus-column-right" },
	{ key = "Mod+H", action = "focus-column-left" },
	{ key = "Mod+L", action = "focus-column-right" },
	{ key = "Mod+Down", action = "focus-window-down" },
	{ key = "Mod+Up", action = "focus-window-up" },
	{ key = "Mod+J", action = "focus-window-down" },
	{ key = "Mod+K", action = "focus-window-up" },
	{ key = "Mod+Home", action = "focus-column-first" },
	{ key = "Mod+End", action = "focus-column-last" },
})

-- Window Movement
niri.config.binds:add({
	{ key = "Mod+Ctrl+Left", action = "move-column-left" },
	{ key = "Mod+Ctrl+Right", action = "move-column-right" },
	{ key = "Mod+Ctrl+H", action = "move-column-left" },
	{ key = "Mod+Ctrl+L", action = "move-column-right" },
	{ key = "Mod+Ctrl+Down", action = "move-window-down" },
	{ key = "Mod+Ctrl+Up", action = "move-window-up" },
	{ key = "Mod+Ctrl+J", action = "move-window-down" },
	{ key = "Mod+Ctrl+K", action = "move-window-up" },
	{ key = "Mod+Ctrl+Home", action = "move-column-to-first" },
	{ key = "Mod+Ctrl+End", action = "move-column-to-last" },
	{ key = "Mod+BracketLeft", action = "consume-or-expel-window-left" },
	{ key = "Mod+BracketRight", action = "consume-or-expel-window-right" },
	{ key = "Mod+Comma", action = "consume-window-into-column" },
	{ key = "Mod+Period", action = "expel-window-from-column" },
})

-- Monitor Focus
niri.config.binds:add({
	{ key = "Mod+Shift+Left", action = "focus-monitor-left" },
	{ key = "Mod+Shift+Right", action = "focus-monitor-right" },
	{ key = "Mod+Shift+Down", action = "focus-monitor-down" },
	{ key = "Mod+Shift+Up", action = "focus-monitor-up" },
	{ key = "Mod+Shift+H", action = "focus-monitor-left" },
	{ key = "Mod+Shift+L", action = "focus-monitor-right" },
	{ key = "Mod+Shift+J", action = "focus-monitor-down" },
	{ key = "Mod+Shift+K", action = "focus-monitor-up" },
})

-- Monitor Movement
niri.config.binds:add({
	{ key = "Mod+Shift+Ctrl+Left", action = "move-column-to-monitor-left" },
	{ key = "Mod+Shift+Ctrl+Right", action = "move-column-to-monitor-right" },
	{ key = "Mod+Shift+Ctrl+Down", action = "move-column-to-monitor-down" },
	{ key = "Mod+Shift+Ctrl+Up", action = "move-column-to-monitor-up" },
	{ key = "Mod+Shift+Ctrl+H", action = "move-column-to-monitor-left" },
	{ key = "Mod+Shift+Ctrl+L", action = "move-column-to-monitor-right" },
	{ key = "Mod+Shift+Ctrl+J", action = "move-column-to-monitor-down" },
	{ key = "Mod+Shift+Ctrl+K", action = "move-column-to-monitor-up" },
})

-- Workspace Navigation
niri.config.binds:add({
	{ key = "Mod+Page_Down", action = "focus-workspace-down" },
	{ key = "Mod+Page_Up", action = "focus-workspace-up" },
	{ key = "Mod+U", action = "focus-workspace-down" },
	{ key = "Mod+I", action = "focus-workspace-up" },
	{ key = "Mod+1", action = "focus-workspace", args = { "1" } },
	{ key = "Mod+2", action = "focus-workspace", args = { "2" } },
	{ key = "Mod+3", action = "focus-workspace", args = { "3" } },
	{ key = "Mod+4", action = "focus-workspace", args = { "4" } },
	{ key = "Mod+5", action = "focus-workspace", args = { "5" } },
	{ key = "Mod+6", action = "focus-workspace", args = { "6" } },
	{ key = "Mod+7", action = "focus-workspace", args = { "7" } },
	{ key = "Mod+8", action = "focus-workspace", args = { "8" } },
	{ key = "Mod+9", action = "focus-workspace", args = { "9" } },
})

-- Workspace Movement
niri.config.binds:add({
	{ key = "Mod+Ctrl+Page_Down", action = "move-column-to-workspace-down" },
	{ key = "Mod+Ctrl+Page_Up", action = "move-column-to-workspace-up" },
	{ key = "Mod+Ctrl+U", action = "move-column-to-workspace-down" },
	{ key = "Mod+Ctrl+I", action = "move-column-to-workspace-up" },
	{ key = "Mod+Ctrl+1", action = "move-column-to-workspace", args = { "1" } },
	{ key = "Mod+Ctrl+2", action = "move-column-to-workspace", args = { "2" } },
	{ key = "Mod+Ctrl+3", action = "move-column-to-workspace", args = { "3" } },
	{ key = "Mod+Ctrl+4", action = "move-column-to-workspace", args = { "4" } },
	{ key = "Mod+Ctrl+5", action = "move-column-to-workspace", args = { "5" } },
	{ key = "Mod+Ctrl+6", action = "move-column-to-workspace", args = { "6" } },
	{ key = "Mod+Ctrl+7", action = "move-column-to-workspace", args = { "7" } },
	{ key = "Mod+Ctrl+8", action = "move-column-to-workspace", args = { "8" } },
	{ key = "Mod+Ctrl+9", action = "move-column-to-workspace", args = { "9" } },
	{ key = "Mod+Shift+Page_Down", action = "move-workspace-down" },
	{ key = "Mod+Shift+Page_Up", action = "move-workspace-up" },
	{ key = "Mod+Shift+U", action = "move-workspace-down" },
	{ key = "Mod+Shift+I", action = "move-workspace-up" },
})

-- Window Sizing
niri.config.binds:add({
	{ key = "Mod+R", action = "switch-preset-column-width" },
	{ key = "Mod+Shift+R", action = "switch-preset-window-height" },
	{ key = "Mod+Ctrl+R", action = "reset-window-height" },
	{ key = "Mod+Minus", action = "set-column-width", args = { "-10%" } },
	{ key = "Mod+Equal", action = "set-column-width", args = { "+10%" } },
	{ key = "Mod+Shift+Minus", action = "set-window-height", args = { "-10%" } },
	{ key = "Mod+Shift+Equal", action = "set-window-height", args = { "+10%" } },
})

-- Mouse Wheel Bindings
niri.config.binds:add({
	{ key = "Mod+WheelScrollDown", action = "focus-workspace-down", cooldown_ms = 150 },
	{ key = "Mod+WheelScrollUp", action = "focus-workspace-up", cooldown_ms = 150 },
	{ key = "Mod+Ctrl+WheelScrollDown", action = "move-column-to-workspace-down", cooldown_ms = 150 },
	{ key = "Mod+Ctrl+WheelScrollUp", action = "move-column-to-workspace-up", cooldown_ms = 150 },
	{ key = "Mod+WheelScrollRight", action = "focus-column-right" },
	{ key = "Mod+WheelScrollLeft", action = "focus-column-left" },
	{ key = "Mod+Ctrl+WheelScrollRight", action = "move-column-right" },
	{ key = "Mod+Ctrl+WheelScrollLeft", action = "move-column-left" },
	{ key = "Mod+Shift+WheelScrollDown", action = "focus-column-right" },
	{ key = "Mod+Shift+WheelScrollUp", action = "focus-column-left" },
	{ key = "Mod+Ctrl+Shift+WheelScrollDown", action = "move-column-right" },
	{ key = "Mod+Ctrl+Shift+WheelScrollUp", action = "move-column-left" },
})

-- Overview & Screenshots
niri.config.binds:add({
	{ key = "Mod+O", action = "toggle-overview" },
	{ key = "Print", action = "screenshot" },
	{ key = "Ctrl+Print", action = "screenshot-screen" },
	{ key = "Alt+Print", action = "screenshot-window" },
})

-- ============================================================================
-- WINDOW RULES (Collection)
-- ============================================================================

niri.config.window_rules:add({
	match = { app_id = "^org%.wezfurlong%.wezterm$" },
	default_column_width = {},
})

niri.config.window_rules:add({
	match = {
		app_id = "firefox$",
		title = "^Picture-in-Picture$",
	},
	open_floating = true,
})

-- ============================================================================
-- LAYER RULES (Collection)
-- ============================================================================

-- niri.config.layer_rules:add({
--     match = { namespace = "swaync", at_startup = true },
--     block_out_from = "screencast",
-- })

-- ============================================================================
-- APPLY CONFIGURATION
-- ============================================================================
-- Apply all staged changes to the compositor

niri.config:apply()

-- ============================================================================
-- POST-CONFIG: STARTUP COMMANDS
-- ============================================================================
-- Spawn startup applications using the action API (method-style with :)

niri.action:spawn({ "waybar" })
niri.action:spawn({ "swaync" })
niri.schedule(function()
	local kitty = niri.action:spawn({ "kitty" }, {})
	niri.utils.log(kitty)
end)

-- Shell commands with pipes/variables:
-- niri.action:spawn_sh("dbus-update-activation-environment --systemd WAYLAND_DISPLAY")

-- ============================================================================
-- POST-CONFIG: EVENT HANDLERS
-- ============================================================================
-- Register event handlers using the new niri.events API
-- Syntax: niri.events:on(event_name, callback)

-- Log all window opens
niri.events:on("window:open", function(ev)
	niri.utils.log("Window opened: " .. (ev.title or "untitled") .. " [" .. (ev.app_id or "unknown") .. "]")
end)

-- One-shot handler for first window
niri.events:once("window:open", function(ev)
	niri.utils.log("First window opened: " .. (ev.app_id or "unknown"))
end)

-- Monitor workspace changes
niri.events:on("workspace:activate", function(ev)
	niri.utils.log("Workspace activated: " .. (ev.name or tostring(ev.index)))
end)

-- Log window title changes
-- niri.events:on("window:title_changed", function(ev)
-- 	niri.utils.log("Window title changed: " .. (ev.title or ""))
-- end)

-- Log config reloads
niri.events:on("config:reload", function(ev)
	if ev.success then
		niri.utils.log("Configuration reloaded successfully")
	else
		niri.utils.warn("Configuration reload failed")
	end
end)

-- Log overview open/close
niri.events:on("overview:open", function(ev)
	niri.utils.debug("Overview opened")
end)

niri.events:on("overview:close", function(ev)
	niri.utils.debug("Overview closed")
end)

-- ============================================================================
-- POST-CONFIG: RUNTIME QUERIES
-- ============================================================================
-- Query runtime state using niri.state

-- Example: Log current window count
-- niri.events:on("window:title_changed", function(ev)
-- 	niri.utils.log("Current windows:")
-- 	for _, win in ipairs(niri.state.windows()) do
-- 		niri.utils.log(" - " .. (win.title or "untitled") .. " [" .. (win.app_id or "unknown") .. "]")
-- 	end
-- end)

-- Example: Check for multiple monitors
-- local outputs = niri.state.outputs()
-- if #outputs > 1 then
--     niri.utils.log("Multiple monitors detected!")
-- else
-- 	niri.utils.log("Single monitor setup.")
-- end

-- ============================================================================
-- DYNAMIC CONFIGURATION EXAMPLE
-- ============================================================================
-- Adjust layout based on monitor count (requires runtime state access)

-- local outputs = niri.state.outputs()
-- if outputs and #outputs > 1 then
--     -- Wider gaps on multi-monitor setup
--     niri.config.layout.gaps = 20
--     niri.config:apply()
-- end

-- ============================================================================
-- LOG SUCCESS
-- ============================================================================

local binds_count = #niri.config.binds:list()
niri.utils.log("Niri v2 configuration loaded!")
niri.utils.log("Loaded " .. binds_count .. " keybindings")

-- ============================================================================
-- ASYNC PRIMITIVES EXAMPLES
-- ============================================================================
-- These APIs allow non-blocking operations in the compositor

-- ----------------------------------------------------------------------------
-- niri.schedule(fn) - Defer execution to next event loop iteration
-- ----------------------------------------------------------------------------
-- Use this to break up work and avoid blocking the compositor

-- Example: Deferred logging after config loads
-- niri.schedule(function()
-- 	niri.utils.log("Deferred: Config fully loaded at " .. niri.loop.now() .. "ms")
-- end)

-- Example: Break up heavy work in event handlers
niri.events:on("window:open", function(ev)
	-- Quick work synchronously
	local window_id = ev.id

	-- Defer heavy work to not block the window from appearing
	niri.schedule(function()
		-- This runs after the window is shown
		niri.utils.log("Deferred analysis of window: " .. window_id)
	end)
end)

-- ----------------------------------------------------------------------------
-- niri.loop.now() - Get monotonic time in milliseconds
-- ----------------------------------------------------------------------------
-- Useful for timing operations and animations

local startup_time = niri.loop.now()
niri.utils.log("Config evaluation started at: " .. startup_time .. "ms since compositor start")

-- ----------------------------------------------------------------------------
-- niri.loop.new_timer() - Create timers for delayed/repeated execution
-- ----------------------------------------------------------------------------
-- Timers persist until explicitly closed (Neovim model)

-- Example: One-shot timer (runs once after delay)
local delayed_timer = niri.loop.new_timer()
delayed_timer:start(5000, 0, function()
	niri.utils.log("5 seconds after config load!")
	delayed_timer:close() -- Clean up (required!)
end)

-- Example: Repeating timer (runs every N milliseconds)
local tick_count = 0
local repeating_timer = niri.loop.new_timer()
repeating_timer:start(0, 60000, function() -- Every 60 seconds
	tick_count = tick_count + 1
	niri.utils.log("Heartbeat #" .. tick_count)
	-- To stop: repeating_timer:close()
end)

-- Example: Debounced operation (useful for rapid events)
local debounce_timer = niri.loop.new_timer()
local pending_action = nil

local function debounced_log(msg)
	pending_action = msg
	debounce_timer:stop() -- Cancel previous
	debounce_timer:start(100, 0, function() -- 100ms debounce
		if pending_action then
			niri.utils.log("Debounced: " .. pending_action)
			pending_action = nil
		end
	end)
end

-- Example: set_timeout helper (Neovim-style)
local function set_timeout(timeout_ms, callback)
	local timer = niri.loop.new_timer()
	timer:start(timeout_ms, 0, function()
		timer:stop()
		timer:close()
		callback()
	end)
	return timer
end

set_timeout(1000, function()
	niri.utils.log("This runs after 1 second")
end)

-- Example: set_interval helper (Neovim-style)
local function set_interval(interval_ms, callback)
	local timer = niri.loop.new_timer()
	timer:start(0, interval_ms, function()
		callback()
	end)
	return timer
end

local interval = set_interval(2000, function()
	niri.utils.log("Every 2 seconds")
end)
-- Later: interval:close()
set_timeout(20000, function()
	interval:close()
	niri.utils.log("Stopped 2-second interval after 20 seconds")
end)

set_timeout(5000, function()
	niri.config.overview.backdrop_color = "#ff0000aa"
	niri.config:apply()
	niri.utils.log("Changed overview backdrop color after 5 seconds")
end)

-- ============================================================================
-- PRACTICAL ASYNC EXAMPLES
-- ============================================================================

-- Example: Log timing of config load
niri.schedule(function()
	local end_time = niri.loop.now()
	niri.utils.log("Config load completed in " .. (end_time - startup_time) .. "ms")
	debounced_log("Config load timing logged 100ms earlier")
	-- niri.action:spawn({ "waybar" })
	niri.events:once("window:open", function(ev)
		niri.action:focus_window(ev.id)
		niri.action:fullscreen_window()
	end)
	-- niri.action:spawn_sh("kitty nvim")
	-- set_timeout(1000, function()
	-- 	niri.action:fullscreen_window()
	-- 	niri.utils.log("Fullscreened the window after 1 second")
	-- end)
end)
