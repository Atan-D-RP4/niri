-- Example: Comprehensive runtime state query
--
-- This demonstrates all four niri.runtime API functions:
-- - get_windows()
-- - get_focused_window()
-- - get_workspaces()
-- - get_outputs()

niri.log("=== Niri Runtime State ===\n")

-- Query outputs (monitors)
local outputs = niri.runtime.get_outputs()
niri.log(string.format("Outputs: %d connected", #outputs))
for _, output in ipairs(outputs) do
    niri.log(string.format("  - %s (%s %s)", output.name, output.make, output.model))
end

-- Query workspaces
local workspaces = niri.runtime.get_workspaces()
niri.log(string.format("\nWorkspaces: %d total", #workspaces))
for _, ws in ipairs(workspaces) do
    local status = {}
    if ws.is_focused then table.insert(status, "focused") end
    if ws.is_active then table.insert(status, "active") end
    if ws.is_urgent then table.insert(status, "urgent") end
   
    local status_str = #status > 0 and (" [" .. table.concat(status, ", ") .. "]") or ""
    niri.log(string.format("  - %s on %s%s",
        ws.name or string.format("Workspace %d", ws.idx + 1),
        ws.output or "none",
        status_str))
end

-- Query all windows
local windows = niri.runtime.get_windows()
niri.log(string.format("\nWindows: %d open", #windows))

-- Query focused window
local focused = niri.runtime.get_focused_window()
if focused then
    niri.log(string.format("\nFocused Window:"))
    niri.log(string.format("  Title: %s", focused.title or "(no title)"))
    niri.log(string.format("  App ID: %s", focused.app_id or "(unknown)"))
    niri.log(string.format("  Workspace ID: %s", focused.workspace_id or "(none)"))
    if focused.layout then
        niri.log(string.format("  Size: %dx%d",
            focused.layout.window_size[1], focused.layout.window_size[2]))
    end
else
    niri.log("\nNo window currently focused")
end

-- Show summary statistics
niri.log("\n=== Summary ===")
niri.log(string.format("%d outputs, %d workspaces, %d windows",
    #outputs, #workspaces, #windows))

-- Count windows by state
local floating_count = 0
local urgent_count = 0
for _, win in ipairs(windows) do
    if win.is_floating then floating_count = floating_count + 1 end
    if win.is_urgent then urgent_count = urgent_count + 1 end
end
niri.log(string.format("%d floating windows, %d urgent windows",
    floating_count, urgent_count))
