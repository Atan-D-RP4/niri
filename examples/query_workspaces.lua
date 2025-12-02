-- Example: Query workspaces and print information
--
-- This demonstrates the niri.state.workspaces() API which returns
-- an array of all workspaces currently managed by the compositor.

local workspaces = niri.state.workspaces()

niri.utils.log("=== All Workspaces ===")
niri.utils.log(string.format("Total workspaces: %d", #workspaces))

for i, ws in ipairs(workspaces) do
    niri.utils.log(string.format("\nWorkspace %d:", i))
    niri.utils.log(string.format("  ID: %d", ws.id))
    niri.utils.log(string.format("  Index: %d", ws.idx))
    niri.utils.log(string.format("  Name: %s", ws.name or "(unnamed)"))
    niri.utils.log(string.format("  Output: %s", ws.output or "nil"))
    niri.utils.log(string.format("  Active: %s", tostring(ws.is_active)))
    niri.utils.log(string.format("  Focused: %s", tostring(ws.is_focused)))
    niri.utils.log(string.format("  Urgent: %s", tostring(ws.is_urgent)))
    niri.utils.log(string.format("  Active Window ID: %s", ws.active_window_id or "nil"))
end

-- Example: Find the focused workspace
for _, ws in ipairs(workspaces) do
    if ws.is_focused then
        niri.utils.log(string.format("\n>>> Currently focused workspace: %s (ID: %d)",
            ws.name or "(unnamed)", ws.id))
        break
    end
end
