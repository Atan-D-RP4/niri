-- Example: Query all windows and print information
-- 
-- This demonstrates the niri.runtime.get_windows() API which returns
-- an array of all windows currently managed by the compositor.

local windows = niri.runtime.get_windows()

niri.log("=== All Windows ===")
niri.log(string.format("Total windows: %d", #windows))

for i, win in ipairs(windows) do
    niri.log(string.format("\nWindow %d:", i))
    niri.log(string.format("  ID: %d", win.id))
    niri.log(string.format("  Title: %s", win.title or "nil"))
    niri.log(string.format("  App ID: %s", win.app_id or "nil"))
    niri.log(string.format("  Workspace ID: %s", win.workspace_id or "nil"))
    niri.log(string.format("  Focused: %s", tostring(win.is_focused)))
    niri.log(string.format("  Floating: %s", tostring(win.is_floating)))
    niri.log(string.format("  Urgent: %s", tostring(win.is_urgent)))
   
    if win.layout then
        niri.log(string.format("  Size: %dx%d", win.layout.window_size[1], win.layout.window_size[2]))
    end
end
