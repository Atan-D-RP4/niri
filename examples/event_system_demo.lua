#!/usr/bin/env niri
-- Event System Demo
--
-- This example demonstrates the Lua event system in Niri.
--
-- The Niri Lua runtime provides an event system that allows scripts to listen
-- to various compositor events like window opening/closing, workspace switching,
-- monitor connection, and layout changes.
--
-- Supported events (25+ available):
--
-- Lifecycle:
--   startup, shutdown
--
-- Window:
--   window:open, window:close, window:focus, window:blur,
--   window:title_changed, window:app_id_changed,
--   window:fullscreen, window:maximize, window:move, window:resize
--
-- Workspace:
--   workspace:activate, workspace:deactivate,
--   workspace:create, workspace:destroy, workspace:rename
--
-- Output (Monitor):
--   output:connect, output:disconnect, output:mode_change
--
-- Layout:
--   layout:mode_changed, layout:window_added, layout:window_removed
--
-- Config:
--   config:reload
--
-- Overview:
--   overview:open, overview:close
--
-- Lock:
--   lock:activate, lock:deactivate

-- Apply minimal config
niri.config:apply()

-- Track event counts for demonstration
local event_counts = {}

-- Helper function to register event listener
-- Uses the niri.events:on() API (method-style with colon)
local function register_event(event_type)
    niri.events:on(event_type, function(data)
        event_counts[event_type] = (event_counts[event_type] or 0) + 1
        niri.utils.log(string.format("Event: %s (count: %d)", event_type, event_counts[event_type]))
        if data then
            for key, value in pairs(data) do
                niri.utils.log(string.format("  %s: %s", key, tostring(value)))
            end
        end
    end)
end

-- Register all event types
niri.utils.log("=== Niri Event System Demo ===")
niri.utils.log("Setting up event listeners...")

-- Window events
register_event("window:open")
register_event("window:close")
register_event("window:focus")
register_event("window:blur")
register_event("window:title_changed")

-- Workspace events
register_event("workspace:activate")
register_event("workspace:deactivate")
register_event("workspace:create")

-- Output events
register_event("output:connect")
register_event("output:disconnect")
register_event("output:mode_change")

-- Layout events
register_event("layout:mode_changed")
register_event("layout:window_added")
register_event("layout:window_removed")

-- Config events
register_event("config:reload")

-- Overview events
register_event("overview:open")
register_event("overview:close")

niri.utils.log("Event listeners registered. You should see events as they occur.")
niri.utils.log("Try opening/closing windows, switching workspaces, toggling overview, etc.")

-- Demonstrate niri.events:once() - fires only once then auto-removes
niri.events:once("window:open", function(data)
    niri.utils.log(">>> First window opened (this event fired once only)")
    niri.utils.log("    App: " .. (data.app_id or "unknown"))
end)

-- Example: Remove a listener
-- local function my_handler(data)
--     niri.utils.log("Focus: " .. (data.title or ""))
-- end
-- niri.events:on("window:focus", my_handler)
-- Later: niri.events:off("window:focus", my_handler)

-- Print event summary using a timer
local summary_timer = niri.loop.new_timer()
summary_timer:start(60000, 60000, function()  -- Every 60 seconds
    niri.utils.log("=== Event Summary ===")
    for event_type, count in pairs(event_counts) do
        niri.utils.log(string.format("  %s: %d events", event_type, count))
    end
end)

niri.utils.log("Event summary will print every 60 seconds.")
