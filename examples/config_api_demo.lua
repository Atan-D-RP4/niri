#!/usr/bin/env lua
-- Configuration API Demo
-- Demonstrates reading all available Niri configuration settings from Lua

-- Colors for output
local colors = {
    reset = "\27[0m",
    bold = "\27[1m",
    cyan = "\27[36m",
    green = "\27[32m",
    yellow = "\27[33m",
    blue = "\27[34m",
}

local function print_section(title)
    print("\n" .. colors.bold .. colors.cyan .. "=== " .. title .. " ===" .. colors.reset)
end

local function print_subsection(title)
    print(colors.bold .. colors.blue .. "  " .. title .. colors.reset)
end

local function print_value(key, value)
    if type(value) == "boolean" then
        value = value and colors.green .. "true" .. colors.reset or colors.yellow .. "false" .. colors.reset
    elseif type(value) == "string" then
        value = colors.cyan .. '"' .. value .. '"' .. colors.reset
    elseif type(value) == "number" then
        value = colors.yellow .. tostring(value) .. colors.reset
    end
    print(string.format("    %s: %s", key, value))
end

-- Ensure niri.config exists
if not niri or not niri.config then
    print(colors.yellow .. "Warning: niri.config not available" .. colors.reset)
    return
end

print_section("ANIMATIONS CONFIGURATION")
print_subsection("Global Settings")
print_value("off", niri.config.animations.off)
print_value("slowdown", niri.config.animations.slowdown)

print_subsection("Workspace Switch Animation")
if niri.config.animations.workspace_switch then
    print_value("off", niri.config.animations.workspace_switch.off)
    print_value("duration_ms", niri.config.animations.workspace_switch.duration_ms)
    print_value("curve", niri.config.animations.workspace_switch.curve)
end

print_section("INPUT CONFIGURATION")
print_subsection("Keyboard")
if niri.config.input.keyboard then
    print_value("repeat_delay", niri.config.input.keyboard.repeat_delay)
    print_value("repeat_rate", niri.config.input.keyboard.repeat_rate)
    print_value("numlock", niri.config.input.keyboard.numlock)
    if niri.config.input.keyboard.xkb then
        print_value("xkb.layout", niri.config.input.keyboard.xkb.layout)
        print_value("xkb.variant", niri.config.input.keyboard.xkb.variant)
    end
end

print_subsection("Mouse")
if niri.config.input.mouse then
    print_value("accel_speed", niri.config.input.mouse.accel_speed)
    print_value("accel_profile", niri.config.input.mouse.accel_profile)
end

print_subsection("Touchpad")
if niri.config.input.touchpad then
    print_value("accel_speed", niri.config.input.touchpad.accel_speed)
    print_value("tap", niri.config.input.touchpad.tap)
    print_value("natural_scroll", niri.config.input.touchpad.natural_scroll)
end

print_subsection("Trackpoint")
if niri.config.input.trackpoint then
    print_value("accel_speed", niri.config.input.trackpoint.accel_speed)
    print_value("natural_scroll", niri.config.input.trackpoint.natural_scroll)
end

print_subsection("Global Input Options")
if niri.config.input.warp_mouse_to_focus then
    print_value("warp_mouse_to_focus", niri.config.input.warp_mouse_to_focus)
end
if niri.config.input.focus_follows_mouse then
    print_value("focus_follows_mouse (enabled)", true)
end

print_section("LAYOUT CONFIGURATION")
print_value("gaps", niri.config.layout.gaps)
print_value("center_focused_column", niri.config.layout.center_focused_column)
print_value("always_center_single_column", niri.config.layout.always_center_single_column)
print_value("empty_workspace_above_first", niri.config.layout.empty_workspace_above_first)
print_value("default_column_display", niri.config.layout.default_column_display)

print_subsection("Struts")
if niri.config.layout.struts then
    print_value("left", niri.config.layout.struts.left)
    print_value("right", niri.config.layout.struts.right)
    print_value("top", niri.config.layout.struts.top)
    print_value("bottom", niri.config.layout.struts.bottom)
end

print_subsection("Focus Ring")
if niri.config.layout.focus_ring then
    print_value("off", niri.config.layout.focus_ring.off)
    print_value("width", niri.config.layout.focus_ring.width)
    print_value("active_color", niri.config.layout.focus_ring.active_color)
    print_value("inactive_color", niri.config.layout.focus_ring.inactive_color)
    print_value("urgent_color", niri.config.layout.focus_ring.urgent_color)
end

print_subsection("Border")
if niri.config.layout.border then
    print_value("off", niri.config.layout.border.off)
    print_value("width", niri.config.layout.border.width)
    print_value("active_color", niri.config.layout.border.active_color)
end

print_subsection("Shadow")
if niri.config.layout.shadow then
    print_value("on", niri.config.layout.shadow.on)
    print_value("softness", niri.config.layout.shadow.softness)
    print_value("spread", niri.config.layout.shadow.spread)
    print_value("color", niri.config.layout.shadow.color)
    print_value("draw_behind_window", niri.config.layout.shadow.draw_behind_window)
end

print_subsection("Tab Indicator")
if niri.config.layout.tab_indicator then
    print_value("off", niri.config.layout.tab_indicator.off)
    print_value("width", niri.config.layout.tab_indicator.width)
end

print_subsection("Insert Hint")
if niri.config.layout.insert_hint then
    print_value("off", niri.config.layout.insert_hint.off)
    print_value("color", niri.config.layout.insert_hint.color)
end

print_subsection("Preset Column Widths")
if niri.config.layout.preset_column_widths then
    for i, size in ipairs(niri.config.layout.preset_column_widths) do
        print(string.format("    [%d]: %s", i, size))
    end
end

print_section("CURSOR CONFIGURATION")
if niri.config.cursor then
    print_value("xcursor_theme", niri.config.cursor.xcursor_theme)
    print_value("xcursor_size", niri.config.cursor.xcursor_size)
    print_value("hide_when_typing", niri.config.cursor.hide_when_typing)
    if niri.config.cursor.hide_after_inactive_ms then
        print_value("hide_after_inactive_ms", niri.config.cursor.hide_after_inactive_ms)
    end
end

print_section("OUTPUT CONFIGURATION")
if niri.config.output then
    for output_name, config in pairs(niri.config.output) do
        if type(config) == "table" then
            print_subsection("Output: " .. output_name)
            print_value("off", config.off)
            if config.scale then
                print_value("scale", config.scale)
            end
            if config.x then
                print_value("x", config.x)
            end
            if config.y then
                print_value("y", config.y)
            end
        end
    end
end

print_section("GESTURES CONFIGURATION")
if niri.config.gestures then
    print_subsection("Drag & Drop Edge View Scroll")
    if niri.config.gestures.dnd_edge_view_scroll then
        print_value("trigger_width", niri.config.gestures.dnd_edge_view_scroll.trigger_width)
        print_value("delay_ms", niri.config.gestures.dnd_edge_view_scroll.delay_ms)
        print_value("max_speed", niri.config.gestures.dnd_edge_view_scroll.max_speed)
    end

    print_subsection("Drag & Drop Edge Workspace Switch")
    if niri.config.gestures.dnd_edge_workspace_switch then
        print_value("trigger_height", niri.config.gestures.dnd_edge_workspace_switch.trigger_height)
        print_value("delay_ms", niri.config.gestures.dnd_edge_workspace_switch.delay_ms)
    end

    print_subsection("Hot Corners")
    if niri.config.gestures.hot_corners then
        print_value("off", niri.config.gestures.hot_corners.off)
        print_value("top_left", niri.config.gestures.hot_corners.top_left)
        print_value("top_right", niri.config.gestures.hot_corners.top_right)
    end
end

print_section("OVERVIEW CONFIGURATION")
if niri.config.overview then
    print_value("zoom", niri.config.overview.zoom)
    print_value("backdrop_color", niri.config.overview.backdrop_color)
    
    if niri.config.overview.workspace_shadow then
        print_subsection("Workspace Shadow")
        print_value("off", niri.config.overview.workspace_shadow.off)
        print_value("softness", niri.config.overview.workspace_shadow.softness)
        print_value("spread", niri.config.overview.workspace_shadow.spread)
    end
end

print_section("DEBUG CONFIGURATION")
if niri.config.debug then
    print_subsection("Debug Options")
    print_value("enable_overlay_planes", niri.config.debug.enable_overlay_planes)
    print_value("disable_cursor_plane", niri.config.debug.disable_cursor_plane)
    print_value("disable_direct_scanout", niri.config.debug.disable_direct_scanout)
    print_value("disable_transactions", niri.config.debug.disable_transactions)
end

print_section("MISCELLANEOUS CONFIGURATION")
print_subsection("Flags")
print_value("prefer_no_csd", niri.config.prefer_no_csd)

if niri.config.screenshot_path then
    print_value("screenshot_path", niri.config.screenshot_path)
end

print_subsection("Clipboard")
if niri.config.clipboard then
    print_value("disable_primary", niri.config.clipboard.disable_primary)
end

print_subsection("Hotkey Overlay")
if niri.config.hotkey_overlay then
    print_value("skip_at_startup", niri.config.hotkey_overlay.skip_at_startup)
    print_value("hide_not_bound", niri.config.hotkey_overlay.hide_not_bound)
end

print_subsection("Config Notification")
if niri.config.config_notification then
    print_value("disable_failed", niri.config.config_notification.disable_failed)
end

print_subsection("Xwayland Satellite")
if niri.config.xwayland_satellite then
    print_value("off", niri.config.xwayland_satellite.off)
    print_value("path", niri.config.xwayland_satellite.path)
end

print_subsection("Environment Variables")
if niri.config.environment then
    for key, value in pairs(niri.config.environment) do
        if value then
            print_value(key, value)
        else
            print(string.format("    %s: %s", key, colors.yellow .. "(null)" .. colors.reset))
        end
    end
end

print_subsection("Spawn at Startup")
if niri.config.spawn_at_startup then
    for i, cmd_table in ipairs(niri.config.spawn_at_startup) do
        if type(cmd_table) == "table" then
            print(string.format("    [%d]: %s", i, table.concat(cmd_table, " ")))
        end
    end
end

print("\n" .. colors.bold .. colors.green .. "✓ Configuration API Demo Complete" .. colors.reset .. "\n")
