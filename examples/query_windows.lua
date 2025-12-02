-- Example: Query all windows and print information
-- 
-- This demonstrates the niri.state.windows() API which returns
-- an array of all windows currently managed by the compositor.

local windows = niri.state.windows()

niri.utils.log("=== All Windows ===")
niri.utils.log(string.format("Total windows: %d", #windows))

for i, win in ipairs(windows) do
    niri.utils.log(string.format("\nWindow %d:", i))
    niri.utils.log(string.format("  ID: %d", win.id))
    niri.utils.log(string.format("  Title: %s", win.title or "nil"))
    niri.utils.log(string.format("  App ID: %s", win.app_id or "nil"))
    niri.utils.log(string.format("  Workspace ID: %s", win.workspace_id or "nil"))
    niri.utils.log(string.format("  Focused: %s", tostring(win.is_focused)))
    niri.utils.log(string.format("  Floating: %s", tostring(win.is_floating)))
    niri.utils.log(string.format("  Urgent: %s", tostring(win.is_urgent)))
   
    if win.layout then
        niri.utils.log(string.format("  Size: %dx%d", win.layout.window_size[1], win.layout.window_size[2]))
    end
end
