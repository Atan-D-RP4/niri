#!/usr/bin/env niri
--! This example demonstrates the Lua event system in Niri.
--! 
--! The Niri Lua runtime provides an event system that allows scripts to listen
--! to various compositor events like window opening/closing, workspace switching,
--! monitor connection, and layout changes.
--!
--! Supported events:
--! - window:open      - Window created
--! - window:close     - Window destroyed
--! - window:focus     - Window received focus
--! - window:blur      - Window lost focus
--! - workspace:activate   - Workspace became active
--! - workspace:deactivate - Workspace became inactive
--! - monitor:connect      - Monitor connected
--! - monitor:disconnect   - Monitor disconnected
--! - layout:mode_changed  - Tiling/floating mode changed
--! - layout:window_added  - Window added to layout
--! - layout:window_removed - Window removed from layout

niri.apply_config({
    -- Configuration goes here
})

-- Track event counts for demonstration
local event_counts = {}

-- Helper function to register event listener
function register_event(event_type)
    niri.on(event_type, function(data)
        event_counts[event_type] = (event_counts[event_type] or 0) + 1
        niri.log(string.format("Event: %s (count: %d)", event_type, event_counts[event_type]))
        if data then
            for key, value in pairs(data) do
                niri.log(string.format("  %s: %s", key, tostring(value)))
            end
        end
    end)
end

-- Register all event types
niri.log("=== Niri Event System Demo ===")
niri.log("Setting up event listeners...")

register_event("window:open")
register_event("window:close")
register_event("window:focus")
register_event("window:blur")
register_event("workspace:activate")
register_event("workspace:deactivate")
register_event("monitor:connect")
register_event("monitor:disconnect")
register_event("layout:mode_changed")
register_event("layout:window_added")
register_event("layout:window_removed")

niri.log("Event listeners registered. You should see events as they occur.")
niri.log("Try opening/closing windows, switching workspaces, toggling floating mode, etc.")

-- Optional: Register a one-time event listener to demonstrate niri.once()
niri.once("window:open", function(data)
    niri.log(">>> First window opened (this event fired once only)")
end)

-- Print event summary every 60 seconds
--[[ Uncomment to enable periodic summary
local function print_summary()
    niri.log("=== Event Summary ===")
    for event_type, count in pairs(event_counts) do
        niri.log(string.format("%s: %d events", event_type, count))
    end
end
]]
