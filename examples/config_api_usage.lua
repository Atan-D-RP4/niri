-- Example: Reading Configuration API
-- This script demonstrates how to access all Niri configuration settings from Lua
-- All settings are read-only and provide the current configuration state

-- ============================================================================
-- ANIMATIONS CONFIGURATION
-- ============================================================================

-- Access animation settings
local anim_off = niri.config.animations.off
local anim_slowdown = niri.config.animations.slowdown

niri.utils.log("Animations off: " .. tostring(anim_off))
niri.utils.log("Animation slowdown: " .. anim_slowdown)

-- Access specific animation types
local ws_switch_anim = niri.config.animations.workspace_switch
niri.utils.log("Workspace switch animation duration: " .. (ws_switch_anim.duration_ms or "spring"))

local window_open_anim = niri.config.animations.window_open
niri.utils.log("Window open curve: " .. (window_open_anim.curve or "spring"))

-- All available animation types:
-- - workspace_switch
-- - window_open
-- - window_close
-- - horizontal_view_movement
-- - window_movement
-- - window_resize
-- - config_notification_open_close
-- - exit_confirmation_open_close
-- - screenshot_ui_open
-- - overview_open_close

-- ============================================================================
-- INPUT CONFIGURATION
-- ============================================================================

-- Keyboard settings
local kb_layout = niri.config.input.keyboard.xkb.layout
local kb_variant = niri.config.input.keyboard.xkb.variant
local repeat_delay = niri.config.input.keyboard.repeat_delay
local repeat_rate = niri.config.input.keyboard.repeat_rate

niri.utils.log("Keyboard layout: " .. kb_layout)
niri.utils.log("Repeat delay: " .. repeat_delay .. "ms, rate: " .. repeat_rate)

-- Mouse settings
local mouse_accel = niri.config.input.mouse.accel_speed
local mouse_profile = niri.config.input.mouse.accel_profile
niri.utils.log("Mouse acceleration: " .. mouse_accel .. ", profile: " .. mouse_profile)

-- Touchpad settings
local touchpad = niri.config.input.touchpad
niri.utils.log("Touchpad tap enabled: " .. tostring(touchpad.tap))
niri.utils.log("Touchpad natural scroll: " .. tostring(touchpad.natural_scroll))
niri.utils.log("Touchpad accel speed: " .. touchpad.accel_speed)

-- Trackpoint settings (NEW - was previously missing)
local trackpoint = niri.config.input.trackpoint
niri.utils.log("Trackpoint accel speed: " .. trackpoint.accel_speed)
niri.utils.log("Trackpoint natural scroll: " .. tostring(trackpoint.natural_scroll))

-- Global input options (NEW)
if niri.config.input.warp_mouse_to_focus then
    niri.utils.log("Warp mouse to focus: " .. niri.config.input.warp_mouse_to_focus)
end

if niri.config.input.focus_follows_mouse then
    niri.utils.log("Focus follows mouse enabled with scroll amount: " .. 
        (niri.config.input.focus_follows_mouse.max_scroll_amount or "default"))
end

-- ============================================================================
-- LAYOUT CONFIGURATION
-- ============================================================================

-- Basic layout settings
local gaps = niri.config.layout.gaps
niri.utils.log("Gaps: " .. gaps .. "px")

-- Struts (NEW - previously empty placeholder)
local struts = niri.config.layout.struts
niri.utils.log(string.format("Struts - left: %d, right: %d, top: %d, bottom: %d",
    struts.left, struts.right, struts.top, struts.bottom))

-- Focus ring (NEW - now fully exposed)
local focus_ring = niri.config.layout.focus_ring
niri.utils.log("Focus ring off: " .. tostring(focus_ring.off))
niri.utils.log("Focus ring width: " .. focus_ring.width)
niri.utils.log("Focus ring active color: " .. focus_ring.active_color)
niri.utils.log("Focus ring inactive color: " .. focus_ring.inactive_color)
niri.utils.log("Focus ring urgent color: " .. focus_ring.urgent_color)

-- Border (NEW - now fully exposed)
local border = niri.config.layout.border
niri.utils.log("Border off: " .. tostring(border.off))
niri.utils.log("Border width: " .. border.width)
niri.utils.log("Border active color: " .. border.active_color)

-- Shadow (NEW - now fully exposed)
local shadow = niri.config.layout.shadow
niri.utils.log("Shadow on: " .. tostring(shadow.on))
niri.utils.log("Shadow softness: " .. shadow.softness)
niri.utils.log("Shadow spread: " .. shadow.spread)
niri.utils.log(string.format("Shadow offset: x=%d, y=%d", shadow.offset.x, shadow.offset.y))
niri.utils.log("Shadow color: " .. shadow.color)
niri.utils.log("Shadow draw behind window: " .. tostring(shadow.draw_behind_window))

-- Tab indicator (NEW - now fully exposed)
local tab_indicator = niri.config.layout.tab_indicator
niri.utils.log("Tab indicator off: " .. tostring(tab_indicator.off))
niri.utils.log("Tab indicator width: " .. tab_indicator.width)
if tab_indicator.active_color then
    niri.utils.log("Tab indicator active color: " .. tab_indicator.active_color)
end

-- Insert hint (NEW - now fully exposed)
local insert_hint = niri.config.layout.insert_hint
niri.utils.log("Insert hint off: " .. tostring(insert_hint.off))
niri.utils.log("Insert hint color: " .. insert_hint.color)

-- Column and window settings
local center_mode = niri.config.layout.center_focused_column
niri.utils.log("Center focused column: " .. center_mode)

niri.utils.log("Always center single column: " .. 
    tostring(niri.config.layout.always_center_single_column))
niri.utils.log("Empty workspace above first: " .. 
    tostring(niri.config.layout.empty_workspace_above_first))

local default_display = niri.config.layout.default_column_display
niri.utils.log("Default column display: " .. default_display)

-- Preset column widths
local preset_widths = niri.config.layout.preset_column_widths
niri.utils.log("Preset column widths: " .. table.concat(preset_widths, ", "))

-- Default column width
if niri.config.layout.default_column_width then
    niri.utils.log("Default column width: " .. niri.config.layout.default_column_width)
end

-- Preset window heights
local preset_heights = niri.config.layout.preset_window_heights
niri.utils.log("Preset window heights: " .. table.concat(preset_heights, ", "))

-- Background color
niri.utils.log("Background color: " .. niri.config.layout.background_color)

-- ============================================================================
-- CURSOR CONFIGURATION
-- ============================================================================

local cursor = niri.config.cursor
niri.utils.log("Cursor theme: " .. cursor.xcursor_theme)
niri.utils.log("Cursor size: " .. cursor.xcursor_size)
niri.utils.log("Hide cursor when typing: " .. tostring(cursor.hide_when_typing))
if cursor.hide_after_inactive_ms then
    niri.utils.log("Hide cursor after " .. cursor.hide_after_inactive_ms .. "ms of inactivity")
end

-- ============================================================================
-- OUTPUT CONFIGURATION
-- ============================================================================

-- Access per-output settings
local outputs = niri.config.output
for output_name, output_config in pairs(outputs) do
    niri.utils.log("Output: " .. output_name)
    niri.utils.log("  Off: " .. tostring(output_config.off))
    if output_config.scale then
        niri.utils.log("  Scale: " .. output_config.scale)
    end
    if output_config.x then
        niri.utils.log(string.format("  Position: %d, %d", output_config.x, output_config.y))
    end
    if output_config.mode_custom then
        niri.utils.log("  Custom mode: " .. tostring(output_config.mode_custom))
    end
end

-- ============================================================================
-- GESTURES CONFIGURATION
-- ============================================================================

local gestures = niri.config.gestures

-- Drag & drop edge view scroll
local dnd_view = gestures.dnd_edge_view_scroll
niri.utils.log(string.format("DnD edge view scroll - trigger: %d, delay: %d, max speed: %d",
    dnd_view.trigger_width, dnd_view.delay_ms, dnd_view.max_speed))

-- Drag & drop edge workspace switch
local dnd_ws = gestures.dnd_edge_workspace_switch
niri.utils.log(string.format("DnD edge workspace switch - trigger: %d, delay: %d, max speed: %d",
    dnd_ws.trigger_height, dnd_ws.delay_ms, dnd_ws.max_speed))

-- Hot corners
local corners = gestures.hot_corners
niri.utils.log(string.format("Hot corners - TL: %s, TR: %s, BL: %s, BR: %s",
    tostring(corners.top_left), tostring(corners.top_right),
    tostring(corners.bottom_left), tostring(corners.bottom_right)))

-- ============================================================================
-- OVERVIEW CONFIGURATION (NEW)
-- ============================================================================

local overview = niri.config.overview
niri.utils.log("Overview zoom: " .. overview.zoom)
niri.utils.log("Overview backdrop color: " .. overview.backdrop_color)

local ws_shadow = overview.workspace_shadow
niri.utils.log("Overview workspace shadow:")
niri.utils.log("  Off: " .. tostring(ws_shadow.off))
niri.utils.log("  Softness: " .. ws_shadow.softness)
niri.utils.log("  Spread: " .. ws_shadow.spread)
niri.utils.log(string.format("  Offset: x=%d, y=%d", ws_shadow.offset.x, ws_shadow.offset.y))
niri.utils.log("  Color: " .. ws_shadow.color)

-- ============================================================================
-- DEBUG CONFIGURATION (NEW)
-- ============================================================================

local debug = niri.config.debug
if debug.preview_render then
    niri.utils.log("Debug preview render: " .. debug.preview_render)
end
niri.utils.log("Debug enable overlay planes: " .. tostring(debug.enable_overlay_planes))
niri.utils.log("Debug disable direct scanout: " .. tostring(debug.disable_direct_scanout))
niri.utils.log("Debug disable cursor plane: " .. tostring(debug.disable_cursor_plane))

if debug.render_drm_device then
    niri.utils.log("Debug render DRM device: " .. debug.render_drm_device)
end

if #debug.ignored_drm_devices > 0 then
    niri.utils.log("Debug ignored DRM devices:")
    for i, device in ipairs(debug.ignored_drm_devices) do
        niri.utils.log("  " .. i .. ": " .. device)
    end
end

-- ============================================================================
-- CLIPBOARD CONFIGURATION (NEW)
-- ============================================================================

local clipboard = niri.config.clipboard
niri.utils.log("Clipboard disable primary: " .. tostring(clipboard.disable_primary))

-- ============================================================================
-- HOTKEY OVERLAY CONFIGURATION (NEW)
-- ============================================================================

local hotkey = niri.config.hotkey_overlay
niri.utils.log("Hotkey overlay skip at startup: " .. tostring(hotkey.skip_at_startup))
niri.utils.log("Hotkey overlay hide not bound: " .. tostring(hotkey.hide_not_bound))

-- ============================================================================
-- CONFIG NOTIFICATION CONFIGURATION (NEW)
-- ============================================================================

local notif = niri.config.config_notification
niri.utils.log("Config notification disable failed: " .. tostring(notif.disable_failed))

-- ============================================================================
-- XWAYLAND SATELLITE CONFIGURATION (NEW)
-- ============================================================================

local xwayland = niri.config.xwayland_satellite
niri.utils.log("Xwayland satellite off: " .. tostring(xwayland.off))
niri.utils.log("Xwayland satellite path: " .. xwayland.path)

-- ============================================================================
-- MISCELLANEOUS CONFIGURATION (NEW)
-- ============================================================================

-- Spawn at startup
if #niri.config.spawn_at_startup > 0 then
    niri.utils.log("Spawn at startup commands:")
    for i, cmd_array in ipairs(niri.config.spawn_at_startup) do
        niri.utils.log("  " .. i .. ": " .. table.concat(cmd_array, " "))
    end
end

-- Spawn sh at startup
if #niri.config.spawn_sh_at_startup > 0 then
    niri.utils.log("Spawn sh at startup commands:")
    for i, cmd in ipairs(niri.config.spawn_sh_at_startup) do
        niri.utils.log("  " .. i .. ": " .. cmd)
    end
end

-- Prefer no CSD
niri.utils.log("Prefer no CSD: " .. tostring(niri.config.prefer_no_csd))

-- Screenshot path
if niri.config.screenshot_path then
    niri.utils.log("Screenshot path: " .. niri.config.screenshot_path)
end

-- Environment variables
niri.utils.log("Environment variables:")
for var_name, var_value in pairs(niri.config.environment) do
    if var_value then
        niri.utils.log("  " .. var_name .. " = " .. var_value)
    else
        niri.utils.log("  " .. var_name .. " = <unset>")
    end
end

-- ============================================================================
-- PRACTICAL EXAMPLES
-- ============================================================================

-- Example 1: Create a status widget showing current config
function print_config_summary()
    niri.utils.log("=== NIRI CONFIGURATION SUMMARY ===")
    niri.utils.log(string.format("Gaps: %dpx | Focus ring: %s | Border: %s",
        niri.config.layout.gaps,
        tostring(not niri.config.layout.focus_ring.off),
        tostring(not niri.config.layout.border.off)))
    niri.utils.log(string.format("Animations: %s | Cursor theme: %s",
        tostring(not niri.config.animations.off),
        niri.config.cursor.xcursor_theme))
    niri.utils.log("===================================")
end

print_config_summary()

-- Example 2: Validate configuration settings
function validate_config()
    local issues = {}
    
    if niri.config.layout.gaps < 0 then
        table.insert(issues, "Invalid gap size")
    end
    
    if niri.config.layout.focus_ring.width < 0 then
        table.insert(issues, "Invalid focus ring width")
    end
    
    if niri.config.cursor.xcursor_size < 1 or niri.config.cursor.xcursor_size > 256 then
        table.insert(issues, "Invalid cursor size")
    end
    
    if #issues == 0 then
        niri.utils.log("✓ Configuration is valid")
    else
        niri.utils.log("✗ Configuration issues found:")
        for _, issue in ipairs(issues) do
            niri.utils.log("  - " .. issue)
        end
    end
end

validate_config()

-- Example 3: Compare animation settings
function check_animations()
    local anims = niri.config.animations
    local enabled_count = 0
    
    for key, anim in pairs(anims) do
        if type(anim) == "table" and anim.off == false then
            enabled_count = enabled_count + 1
        end
    end
    
    niri.utils.log("Enabled animations: " .. enabled_count)
end

check_animations()
