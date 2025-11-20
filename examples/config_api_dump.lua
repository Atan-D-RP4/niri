-- Example: Read and Display Niri Configuration
-- This script demonstrates the new niri.config API for reading all configuration settings

-- Helper function to print configuration values
local function print_config_section(title, table_data)
    niri.log("=" .. string.rep("=", 70))
    niri.log("  " .. title)
    niri.log("=" .. string.rep("=", 70))
    
    for key, value in pairs(table_data) do
        if type(value) == "table" then
            niri.log("  " .. key .. ":")
            for sub_key, sub_value in pairs(value) do
                if type(sub_value) == "table" then
                    niri.log("    " .. sub_key .. ": {...}")
                else
                    niri.log("    " .. sub_key .. ": " .. tostring(sub_value))
                end
            end
        else
            niri.log("  " .. key .. ": " .. tostring(value))
        end
    end
end

-- 1. Display Animation Configuration
niri.log("\n\n>>> ANIMATIONS CONFIGURATION <<<")
niri.log("Global Settings:")
niri.log("  Off: " .. tostring(niri.config.animations.off))
niri.log("  Slowdown: " .. niri.config.animations.slowdown)

niri.log("\nWorkspace Switch Animation:")
niri.log("  Off: " .. tostring(niri.config.animations.workspace_switch.off))
niri.log("  Duration: " .. niri.config.animations.workspace_switch.duration_ms .. "ms")
niri.log("  Curve: " .. niri.config.animations.workspace_switch.curve)

niri.log("\nWindow Open Animation:")
niri.log("  Off: " .. tostring(niri.config.animations.window_open.off))
niri.log("  Duration: " .. niri.config.animations.window_open.duration_ms .. "ms")

-- 2. Display Input Configuration
niri.log("\n\n>>> INPUT CONFIGURATION <<<")
niri.log("Keyboard:")
niri.log("  Layout: " .. niri.config.input.keyboard.xkb.layout)
niri.log("  Variant: " .. niri.config.input.keyboard.xkb.variant)
niri.log("  Repeat Delay: " .. niri.config.input.keyboard.repeat_delay .. "ms")
niri.log("  Repeat Rate: " .. niri.config.input.keyboard.repeat_rate .. " chars/sec")
niri.log("  Numlock: " .. tostring(niri.config.input.keyboard.numlock))

niri.log("\nMouse:")
niri.log("  Accel Speed: " .. niri.config.input.mouse.accel_speed)
niri.log("  Accel Profile: " .. niri.config.input.mouse.accel_profile)

niri.log("\nTouchpad:")
niri.log("  Accel Speed: " .. niri.config.input.touchpad.accel_speed)
niri.log("  Tap: " .. tostring(niri.config.input.touchpad.tap))
niri.log("  Natural Scroll: " .. tostring(niri.config.input.touchpad.natural_scroll))

niri.log("\nTrackpoint:")
niri.log("  Accel Speed: " .. niri.config.input.trackpoint.accel_speed)
niri.log("  Accel Profile: " .. niri.config.input.trackpoint.accel_profile)
niri.log("  Natural Scroll: " .. tostring(niri.config.input.trackpoint.natural_scroll))

-- 3. Display Layout Configuration
niri.log("\n\n>>> LAYOUT CONFIGURATION <<<")
niri.log("Basic:")
niri.log("  Gaps: " .. niri.config.layout.gaps .. "px")
niri.log("  Background Color: " .. niri.config.layout.background_color)

niri.log("\nStruts (reserved screen edges):")
niri.log("  Left: " .. niri.config.layout.struts.left .. "px")
niri.log("  Right: " .. niri.config.layout.struts.right .. "px")
niri.log("  Top: " .. niri.config.layout.struts.top .. "px")
niri.log("  Bottom: " .. niri.config.layout.struts.bottom .. "px")

niri.log("\nFocus Ring:")
niri.log("  Off: " .. tostring(niri.config.layout.focus_ring.off))
niri.log("  Width: " .. niri.config.layout.focus_ring.width .. "px")
niri.log("  Active Color: " .. niri.config.layout.focus_ring.active_color)
niri.log("  Inactive Color: " .. niri.config.layout.focus_ring.inactive_color)
niri.log("  Urgent Color: " .. niri.config.layout.focus_ring.urgent_color)

niri.log("\nBorder:")
niri.log("  Off: " .. tostring(niri.config.layout.border.off))
niri.log("  Width: " .. niri.config.layout.border.width .. "px")
niri.log("  Active Color: " .. niri.config.layout.border.active_color)
niri.log("  Inactive Color: " .. niri.config.layout.border.inactive_color)

niri.log("\nShadow:")
niri.log("  On: " .. tostring(niri.config.layout.shadow.on))
niri.log("  Softness: " .. niri.config.layout.shadow.softness)
niri.log("  Spread: " .. niri.config.layout.shadow.spread)
niri.log("  Color: " .. niri.config.layout.shadow.color)
niri.log("  Draw Behind: " .. tostring(niri.config.layout.shadow.draw_behind_window))

niri.log("\nTab Indicator:")
niri.log("  Off: " .. tostring(niri.config.layout.tab_indicator.off))
niri.log("  Width: " .. niri.config.layout.tab_indicator.width .. "px")

niri.log("\nColumn Settings:")
niri.log("  Center Focused: " .. niri.config.layout.center_focused_column)
niri.log("  Always Center Single: " .. tostring(niri.config.layout.always_center_single_column))
niri.log("  Default Display: " .. niri.config.layout.default_column_display)

-- 4. Display Cursor Configuration
niri.log("\n\n>>> CURSOR CONFIGURATION <<<")
niri.log("Theme: " .. niri.config.cursor.xcursor_theme)
niri.log("Size: " .. niri.config.cursor.xcursor_size)
niri.log("Hide When Typing: " .. tostring(niri.config.cursor.hide_when_typing))
if niri.config.cursor.hide_after_inactive_ms then
    niri.log("Hide After Inactive: " .. niri.config.cursor.hide_after_inactive_ms .. "ms")
end

-- 5. Display Output Configuration
niri.log("\n\n>>> OUTPUT CONFIGURATION <<<")
if niri.config.output then
    for output_name, output_config in pairs(niri.config.output) do
        niri.log("Output: " .. output_name)
        niri.log("  Off: " .. tostring(output_config.off))
        if output_config.scale then
            niri.log("  Scale: " .. output_config.scale)
        end
        if output_config.x then
            niri.log("  Position: (" .. output_config.x .. ", " .. output_config.y .. ")")
        end
    end
end

-- 6. Display Gestures Configuration
niri.log("\n\n>>> GESTURES CONFIGURATION <<<")
niri.log("Drag & Drop Edge View Scroll:")
niri.log("  Trigger Width: " .. niri.config.gestures.dnd_edge_view_scroll.trigger_width)
niri.log("  Delay: " .. niri.config.gestures.dnd_edge_view_scroll.delay_ms .. "ms")
niri.log("  Max Speed: " .. niri.config.gestures.dnd_edge_view_scroll.max_speed)

niri.log("\nDrag & Drop Edge Workspace Switch:")
niri.log("  Trigger Height: " .. niri.config.gestures.dnd_edge_workspace_switch.trigger_height)
niri.log("  Delay: " .. niri.config.gestures.dnd_edge_workspace_switch.delay_ms .. "ms")

niri.log("\nHot Corners:")
niri.log("  Top Left: " .. tostring(niri.config.gestures.hot_corners.top_left))
niri.log("  Top Right: " .. tostring(niri.config.gestures.hot_corners.top_right))
niri.log("  Bottom Left: " .. tostring(niri.config.gestures.hot_corners.bottom_left))
niri.log("  Bottom Right: " .. tostring(niri.config.gestures.hot_corners.bottom_right))

-- 7. Display Overview Configuration
niri.log("\n\n>>> OVERVIEW CONFIGURATION <<<")
niri.log("Zoom: " .. niri.config.overview.zoom)
niri.log("Backdrop Color: " .. niri.config.overview.backdrop_color)
niri.log("Workspace Shadow:")
niri.log("  Off: " .. tostring(niri.config.overview.workspace_shadow.off))
niri.log("  Softness: " .. niri.config.overview.workspace_shadow.softness)
niri.log("  Spread: " .. niri.config.overview.workspace_shadow.spread)
niri.log("  Color: " .. niri.config.overview.workspace_shadow.color)

-- 8. Display Miscellaneous Configuration
niri.log("\n\n>>> MISCELLANEOUS CONFIGURATION <<<")
niri.log("Prefer No CSD: " .. tostring(niri.config.prefer_no_csd))
if niri.config.screenshot_path then
    niri.log("Screenshot Path: " .. niri.config.screenshot_path)
end

niri.log("\nSpawn at Startup Commands:")
for i, cmd in ipairs(niri.config.spawn_at_startup) do
    niri.log("  " .. i .. ": " .. table.concat(cmd, " "))
end

niri.log("\nSpawn Shell at Startup Commands:")
for i, cmd in ipairs(niri.config.spawn_sh_at_startup) do
    niri.log("  " .. i .. ": " .. cmd)
end

-- 9. Display Clipboard Configuration
niri.log("\n\n>>> CLIPBOARD CONFIGURATION <<<")
niri.log("Disable Primary: " .. tostring(niri.config.clipboard.disable_primary))

-- 10. Display Hotkey Overlay Configuration
niri.log("\n\n>>> HOTKEY OVERLAY CONFIGURATION <<<")
niri.log("Skip at Startup: " .. tostring(niri.config.hotkey_overlay.skip_at_startup))
niri.log("Hide Not Bound: " .. tostring(niri.config.hotkey_overlay.hide_not_bound))

-- 11. Display Config Notification Configuration
niri.log("\n\n>>> CONFIG NOTIFICATION CONFIGURATION <<<")
niri.log("Disable Failed: " .. tostring(niri.config.config_notification.disable_failed))

-- 12. Display Xwayland Satellite Configuration
niri.log("\n\n>>> XWAYLAND SATELLITE CONFIGURATION <<<")
niri.log("Off: " .. tostring(niri.config.xwayland_satellite.off))
niri.log("Path: " .. niri.config.xwayland_satellite.path)

-- 13. Display Debug Configuration
niri.log("\n\n>>> DEBUG CONFIGURATION <<<")
niri.log("Enable Overlay Planes: " .. tostring(niri.config.debug.enable_overlay_planes))
niri.log("Disable Cursor Plane: " .. tostring(niri.config.debug.disable_cursor_plane))
niri.log("Disable Direct Scanout: " .. tostring(niri.config.debug.disable_direct_scanout))
niri.log("Keep Max BPC Unchanged: " .. tostring(niri.config.debug.keep_max_bpc_unchanged))
niri.log("Disable Resize Throttling: " .. tostring(niri.config.debug.disable_resize_throttling))
niri.log("Disable Transactions: " .. tostring(niri.config.debug.disable_transactions))

-- 14. Display Environment Configuration
niri.log("\n\n>>> ENVIRONMENT CONFIGURATION <<<")
niri.log("Environment Variables:")
for key, value in pairs(niri.config.environment) do
    if value then
        niri.log("  " .. key .. " = " .. value)
    else
        niri.log("  " .. key .. " = (unset)")
    end
end

niri.log("\n\n>>> CONFIGURATION DUMP COMPLETE <<<\n")
