-- Example: Query workspaces and print information
--
-- This demonstrates the niri.runtime.get_workspaces() API which returns
-- an array of all workspaces currently managed by the compositor.

local workspaces = niri.runtime.get_workspaces()

niri.log("=== All Workspaces ===")
niri.log(string.format("Total workspaces: %d", #workspaces))

for i, ws in ipairs(workspaces) do
    niri.log(string.format("\nWorkspace %d:", i))
    niri.log(string.format("  ID: %d", ws.id))
    niri.log(string.format("  Index: %d", ws.idx))
    niri.log(string.format("  Name: %s", ws.name or "(unnamed)"))
    niri.log(string.format("  Output: %s", ws.output or "nil"))
    niri.log(string.format("  Active: %s", tostring(ws.is_active)))
    niri.log(string.format("  Focused: %s", tostring(ws.is_focused)))
    niri.log(string.format("  Urgent: %s", tostring(ws.is_urgent)))
    niri.log(string.format("  Active Window ID: %s", ws.active_window_id or "nil"))
end

-- Example: Find the focused workspace
for _, ws in ipairs(workspaces) do
    if ws.is_focused then
        niri.log(string.format("\n>>> Currently focused workspace: %s (ID: %d)", 
            ws.name or "(unnamed)", ws.id))
        break
    end
end
