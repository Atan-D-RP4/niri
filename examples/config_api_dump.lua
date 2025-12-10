-- Example: Read and Display Niri Configuration
-- This script demonstrates the new niri.config API for reading all configuration settings

-- Helper function to print configuration values
local function print_config_section(title, table_data)
    niri.utils.log("=" .. string.rep("=", 70))
    niri.utils.log("  " .. title)
    niri.utils.log("=" .. string.rep("=", 70))
    
    for key, value in pairs(table_data) do
        if type(value) == "table" then
            niri.utils.log("  " .. key .. ":")
            for sub_key, sub_value in pairs(value) do
                if type(sub_value) == "table" then
                    niri.utils.log("    " .. sub_key .. ": {...}")
                else
                    niri.utils.log("    " .. sub_key .. ": " .. tostring(sub_value))
                end
            end
        else
            niri.utils.log("  " .. key .. ": " .. tostring(value))
        end
    end
end

-- 1. Display Animation Configuration
niri.utils.log("\n\n>>> ANIMATIONS CONFIGURATION <<<")
niri.utils.log("Global Settings:")
niri.utils.log("  Off: " .. tostring(niri.config.animations.off))
niri.utils.log("  Slowdown: " .. niri.config.animations.slowdown)

niri.utils.log("\nWorkspace Switch Animation:")
niri.utils.log("  Off: " .. tostring(niri.config.animations.workspace_switch.off))
niri.utils.log("  Duration: " .. niri.config.animations.workspace_switch.duration_ms .. "ms")
niri.utils.log("  Curve: " .. niri.config.animations.workspace_switch.curve)

niri.utils.log("\nWindow Open Animation:")
niri.utils.log("  Off: " .. tostring(niri.config.animations.window_open.off))
niri.utils.log("  Duration: " .. niri.config.animations.window_open.duration_ms .. "ms")

-- 2. Display Input Configuration
niri.utils.log("\n\n>>> INPUT CONFIGURATION <<<")
niri.utils.log("Keyboard:")
niri.utils.log("  Layout: " .. niri.config.input.keyboard.xkb.layout)
niri.utils.log("  Variant: " .. niri.config.input.keyboard.xkb.variant)
niri.utils.log("  Repeat Delay: " .. niri.config.input.keyboard.repeat_delay .. "ms")
niri.utils.log("  Repeat Rate: " .. niri.config.input.keyboard.repeat_rate .. " chars/sec")
niri.utils.log("  Numlock: " .. tostring(niri.config.input.keyboard.numlock))

niri.utils.log("\nMouse:")
niri.utils.log("  Accel Speed: " .. niri.config.input.mouse.accel_speed)
niri.utils.log("  Accel Profile: " .. niri.config.input.mouse.accel_profile)

niri.utils.log("\nTouchpad:")
niri.utils.log("  Accel Speed: " .. niri.config.input.touchpad.accel_speed)
niri.utils.log("  Tap: " .. tostring(niri.config.input.touchpad.tap))
niri.utils.log("  Natural Scroll: " .. tostring(niri.config.input.touchpad.natural_scroll))

niri.utils.log("\nTrackpoint:")
niri.utils.log("  Accel Speed: " .. niri.config.input.trackpoint.accel_speed)
niri.utils.log("  Accel Profile: " .. niri.config.input.trackpoint.accel_profile)
niri.utils.log("  Natural Scroll: " .. tostring(niri.config.input.trackpoint.natural_scroll))

-- 3. Display Layout Configuration
niri.utils.log("\n\n>>> LAYOUT CONFIGURATION <<<")
niri.utils.log("Basic:")
niri.utils.log("  Gaps: " .. niri.config.layout.gaps .. "px")
niri.utils.log("  Background Color: " .. niri.config.layout.background_color)

niri.utils.log("\nStruts (reserved screen edges):")
niri.utils.log("  Left: " .. niri.config.layout.struts.left .. "px")
niri.utils.log("  Right: " .. niri.config.layout.struts.right .. "px")
niri.utils.log("  Top: " .. niri.config.layout.struts.top .. "px")
niri.utils.log("  Bottom: " .. niri.config.layout.struts.bottom .. "px")

niri.utils.log("\nFocus Ring:")
niri.utils.log("  Off: " .. tostring(niri.config.layout.focus_ring.off))
niri.utils.log("  Width: " .. niri.config.layout.focus_ring.width .. "px")
niri.utils.log("  Active Color: " .. niri.config.layout.focus_ring.active_color)
niri.utils.log("  Inactive Color: " .. niri.config.layout.focus_ring.inactive_color)
niri.utils.log("  Urgent Color: " .. niri.config.layout.focus_ring.urgent_color)

niri.utils.log("\nBorder:")
niri.utils.log("  Off: " .. tostring(niri.config.layout.border.off))
niri.utils.log("  Width: " .. niri.config.layout.border.width .. "px")
niri.utils.log("  Active Color: " .. niri.config.layout.border.active_color)
niri.utils.log("  Inactive Color: " .. niri.config.layout.border.inactive_color)

niri.utils.log("\nShadow:")
niri.utils.log("  On: " .. tostring(niri.config.layout.shadow.on))
niri.utils.log("  Softness: " .. niri.config.layout.shadow.softness)
niri.utils.log("  Spread: " .. niri.config.layout.shadow.spread)
niri.utils.log("  Color: " .. niri.config.layout.shadow.color)
niri.utils.log("  Draw Behind: " .. tostring(niri.config.layout.shadow.draw_behind_window))

niri.utils.log("\nTab Indicator:")
niri.utils.log("  Off: " .. tostring(niri.config.layout.tab_indicator.off))
niri.utils.log("  Width: " .. niri.config.layout.tab_indicator.width .. "px")

niri.utils.log("\nColumn Settings:")
niri.utils.log("  Center Focused: " .. niri.config.layout.center_focused_column)
niri.utils.log("  Always Center Single: " .. tostring(niri.config.layout.always_center_single_column))
niri.utils.log("  Default Display: " .. niri.config.layout.default_column_display)

-- 4. Display Cursor Configuration
niri.utils.log("\n\n>>> CURSOR CONFIGURATION <<<")
niri.utils.log("Theme: " .. niri.config.cursor.xcursor_theme)
niri.utils.log("Size: " .. niri.config.cursor.xcursor_size)
niri.utils.log("Hide When Typing: " .. tostring(niri.config.cursor.hide_when_typing))
if niri.config.cursor.hide_after_inactive_ms then
    niri.utils.log("Hide After Inactive: " .. niri.config.cursor.hide_after_inactive_ms .. "ms")
end

-- 5. Display Output Configuration
niri.utils.log("\n\n>>> OUTPUT CONFIGURATION <<<")
if niri.config.output then
    for output_name, output_config in pairs(niri.config.output) do
        niri.utils.log("Output: " .. output_name)
        niri.utils.log("  Off: " .. tostring(output_config.off))
        if output_config.scale then
            niri.utils.log("  Scale: " .. output_config.scale)
        end
        if output_config.x then
            niri.utils.log("  Position: (" .. output_config.x .. ", " .. output_config.y .. ")")
        end
    end
end

-- 6. Display Gestures Configuration
niri.utils.log("\n\n>>> GESTURES CONFIGURATION <<<")
niri.utils.log("Drag & Drop Edge View Scroll:")
niri.utils.log("  Trigger Width: " .. niri.config.gestures.dnd_edge_view_scroll.trigger_width)
niri.utils.log("  Delay: " .. niri.config.gestures.dnd_edge_view_scroll.delay_ms .. "ms")
niri.utils.log("  Max Speed: " .. niri.config.gestures.dnd_edge_view_scroll.max_speed)

niri.utils.log("\nDrag & Drop Edge Workspace Switch:")
niri.utils.log("  Trigger Height: " .. niri.config.gestures.dnd_edge_workspace_switch.trigger_height)
niri.utils.log("  Delay: " .. niri.config.gestures.dnd_edge_workspace_switch.delay_ms .. "ms")

niri.utils.log("\nHot Corners:")
niri.utils.log("  Top Left: " .. tostring(niri.config.gestures.hot_corners.top_left))
niri.utils.log("  Top Right: " .. tostring(niri.config.gestures.hot_corners.top_right))
niri.utils.log("  Bottom Left: " .. tostring(niri.config.gestures.hot_corners.bottom_left))
niri.utils.log("  Bottom Right: " .. tostring(niri.config.gestures.hot_corners.bottom_right))

-- 7. Display Overview Configuration
niri.utils.log("\n\n>>> OVERVIEW CONFIGURATION <<<")
niri.utils.log("Zoom: " .. niri.config.overview.zoom)
niri.utils.log("Backdrop Color: " .. niri.config.overview.backdrop_color)
niri.utils.log("Workspace Shadow:")
niri.utils.log("  Off: " .. tostring(niri.config.overview.workspace_shadow.off))
niri.utils.log("  Softness: " .. niri.config.overview.workspace_shadow.softness)
niri.utils.log("  Spread: " .. niri.config.overview.workspace_shadow.spread)
niri.utils.log("  Color: " .. niri.config.overview.workspace_shadow.color)

-- 8. Display Miscellaneous Configuration
niri.utils.log("\n\n>>> MISCELLANEOUS CONFIGURATION <<<")
niri.utils.log("Prefer No CSD: " .. tostring(niri.config.prefer_no_csd))
if niri.config.screenshot_path then
    niri.utils.log("Screenshot Path: " .. niri.config.screenshot_path)
end

niri.utils.log("\nSpawn at Startup Commands:")
for i, cmd in ipairs(niri.config.spawn_at_startup) do
    niri.utils.log("  " .. i .. ": " .. table.concat(cmd, " "))
end

niri.utils.log("\nSpawn Shell at Startup Commands:")
for i, cmd in ipairs(niri.config.spawn_sh_at_startup) do
    niri.utils.log("  " .. i .. ": " .. cmd)
end

-- 9. Display Clipboard Configuration
niri.utils.log("\n\n>>> CLIPBOARD CONFIGURATION <<<")
niri.utils.log("Disable Primary: " .. tostring(niri.config.clipboard.disable_primary))

-- 10. Display Hotkey Overlay Configuration
niri.utils.log("\n\n>>> HOTKEY OVERLAY CONFIGURATION <<<")
niri.utils.log("Skip at Startup: " .. tostring(niri.config.hotkey_overlay.skip_at_startup))
niri.utils.log("Hide Not Bound: " .. tostring(niri.config.hotkey_overlay.hide_not_bound))

-- 11. Display Config Notification Configuration
niri.utils.log("\n\n>>> CONFIG NOTIFICATION CONFIGURATION <<<")
niri.utils.log("Disable Failed: " .. tostring(niri.config.config_notification.disable_failed))

-- 12. Display Xwayland Satellite Configuration
niri.utils.log("\n\n>>> XWAYLAND SATELLITE CONFIGURATION <<<")
niri.utils.log("Off: " .. tostring(niri.config.xwayland_satellite.off))
niri.utils.log("Path: " .. niri.config.xwayland_satellite.path)

-- 13. Display Debug Configuration
niri.utils.log("\n\n>>> DEBUG CONFIGURATION <<<")
niri.utils.log("Enable Overlay Planes: " .. tostring(niri.config.debug.enable_overlay_planes))
niri.utils.log("Disable Cursor Plane: " .. tostring(niri.config.debug.disable_cursor_plane))
niri.utils.log("Disable Direct Scanout: " .. tostring(niri.config.debug.disable_direct_scanout))
niri.utils.log("Keep Max BPC Unchanged: " .. tostring(niri.config.debug.keep_max_bpc_unchanged))
niri.utils.log("Disable Resize Throttling: " .. tostring(niri.config.debug.disable_resize_throttling))
niri.utils.log("Disable Transactions: " .. tostring(niri.config.debug.disable_transactions))

-- 14. Display Environment Configuration
niri.utils.log("\n\n>>> ENVIRONMENT CONFIGURATION <<<")
niri.utils.log("Environment Variables:")
for key, value in pairs(niri.config.environment) do
    if value then
        niri.utils.log("  " .. key .. " = " .. value)
    else
        niri.utils.log("  " .. key .. " = (unset)")
    end
end

niri.utils.log("\n\n>>> CONFIGURATION DUMP COMPLETE <<<\n")
