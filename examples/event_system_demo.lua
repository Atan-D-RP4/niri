--[[
    Example Niri Lua Event System Plugin
    
    This plugin demonstrates how to use the Niri event system
    for reactive, event-driven Lua programming.
    
    Demonstrates:
    - Window lifecycle tracking
    - Event handler registration and cleanup
    - One-time handlers with `niri.once()`
    - Persistent handlers with `niri.on()`
    - Multiple event types
]]

local M = {}

-- Track open windows for demonstration
local window_tracker = {
    windows = {},
    total_opened = 0,
    total_closed = 0,
}

-- Example 1: Basic window open handler
function M.track_window_opens()
    niri.on("window:open", function(event)
        window_tracker.total_opened = window_tracker.total_opened + 1
        local window = event.window
        niri.log(string.format(
            "[Window Tracker] Window opened: %s (%s) - Total: %d",
            window.title or "(untitled)",
            window.app_id or "(unknown)",
            window_tracker.total_opened
        ))
    end)
end

-- Example 2: One-time handler (fires only on first event)
function M.first_window_notification()
    niri.once("window:open", function(event)
        niri.log("[Event System] First window opened! This message appears only once.")
    end)
end

-- Example 3: Window closing tracker
function M.track_window_closes()
    niri.on("window:close", function(event)
        window_tracker.total_closed = window_tracker.total_closed + 1
        local window = event.window
        niri.log(string.format(
            "[Window Tracker] Window closed: %s - Total closed: %d",
            window.title or "(untitled)",
            window_tracker.total_closed
        ))
    end)
end

-- Example 4: Multiple event tracking
function M.track_focus_changes()
    niri.on("window:focus", function(event)
        local window = event.window
        local prev = event.previous_window
        
        niri.log(string.format(
            "[Focus] Focus changed to: %s (was: %s)",
            window.title or "(untitled)",
            (prev and prev.title) or "(none)"
        ))
    end)
end

-- Example 5: Workspace-aware automation
function M.workspace_automation()
    niri.on("workspace:activate", function(event)
        local ws = event.workspace
        local prev_ws = event.previous_workspace
        
        niri.log(string.format(
            "[Workspace] Switched to workspace: %s (from %s)",
            ws.name or string.format("Workspace %d", ws.idx),
            (prev_ws and prev_ws.name) or "(none)"
        ))
    end)
end

-- Example 6: Monitor connection handling
function M.monitor_tracking()
    niri.on("monitor:connect", function(event)
        local monitor = event.monitor
        niri.log(string.format(
            "[Monitor] Connected: %s (%s) at scale %.1fx",
            monitor.name,
            monitor.model or "(unknown model)",
            monitor.scale or 1.0
        ))
    end)
    
    niri.on("monitor:disconnect", function(event)
        local monitor = event.monitor
        niri.log(string.format(
            "[Monitor] Disconnected: %s",
            monitor.name
        ))
    end)
end

-- Example 7: Logging tracker state
function M.get_statistics()
    return {
        total_opened = window_tracker.total_opened,
        total_closed = window_tracker.total_closed,
        active_windows = window_tracker.total_opened - window_tracker.total_closed,
    }
end

-- Plugin initialization
function M.init()
    niri.log("[Event System Plugin] Initializing...")
    
    -- Register all event handlers
    M.track_window_opens()
    M.first_window_notification()
    M.track_window_closes()
    M.track_focus_changes()
    M.workspace_automation()
    M.monitor_tracking()
    
    niri.log("[Event System Plugin] Initialization complete!")
    niri.log("Event handlers installed:")
    niri.log("  - Window lifecycle tracking")
    niri.log("  - Focus change monitoring")
    niri.log("  - Workspace activation tracking")
    niri.log("  - Monitor connection/disconnection handling")
end

-- Return public API
return M
