-- Example: Comprehensive runtime state query
--
-- This demonstrates all four niri.state API functions:
-- - windows()
-- - focused_window()
-- - workspaces()
-- - outputs()

niri.utils.log("=== Niri Runtime State ===\n")

-- Query outputs (monitors)
local outputs = niri.state.outputs()
niri.utils.log(string.format("Outputs: %d connected", #outputs))
for _, output in ipairs(outputs) do
    niri.utils.log(string.format("  - %s (%s %s)", output.name, output.make, output.model))
end

-- Query workspaces
local workspaces = niri.state.workspaces()
niri.utils.log(string.format("\nWorkspaces: %d total", #workspaces))
for _, ws in ipairs(workspaces) do
    local status = {}
    if ws.is_focused then table.insert(status, "focused") end
    if ws.is_active then table.insert(status, "active") end
    if ws.is_urgent then table.insert(status, "urgent") end

    local status_str = #status > 0 and (" [" .. table.concat(status, ", ") .. "]") or ""
    niri.utils.log(string.format("  - %s on %s%s",
        ws.name or string.format("Workspace %d", ws.idx + 1),
        ws.output or "none",
        status_str))
end

-- Query all windows
local windows = niri.state.windows()
niri.utils.log(string.format("\nWindows: %d open", #windows))

-- Query focused window
local focused = niri.state.focused_window()
if focused then
    niri.utils.log(string.format("\nFocused Window:"))
    niri.utils.log(string.format("  Title: %s", focused.title or "(no title)"))
    niri.utils.log(string.format("  App ID: %s", focused.app_id or "(unknown)"))
    niri.utils.log(string.format("  Workspace ID: %s", focused.workspace_id or "(none)"))
    if focused.layout then
        niri.utils.log(string.format("  Size: %dx%d",
            focused.layout.window_size[1], focused.layout.window_size[2]))
    end
else
    niri.utils.log("\nNo window currently focused")
end

-- Show summary statistics
niri.utils.log("\n=== Summary ===")
niri.utils.log(string.format("%d outputs, %d workspaces, %d windows",
    #outputs, #workspaces, #windows))

-- Count windows by state
local floating_count = 0
local urgent_count = 0
for _, win in ipairs(windows) do
    if win.is_floating then floating_count = floating_count + 1 end
    if win.is_urgent then urgent_count = urgent_count + 1 end
end
niri.utils.log(string.format("%d floating windows, %d urgent windows",
    floating_count, urgent_count))
