-- Test Runtime State API for Niri (v2 API)
-- This file tests the niri.state API functions

local log = niri.utils.log

log("=== Testing Runtime State API ===")

-- Test 1: windows()
log("\n[Test 1] Testing niri.state.windows()")
local windows = niri.state.windows()
if windows then
    log("windows() succeeded, got " .. #windows .. " windows")
    if #windows > 0 then
        log("First window:")
        local win = windows[1]
        log("  ID: " .. tostring(win.id))
        log("  Title: " .. (win.title or "(no title)"))
        log("  App ID: " .. (win.app_id or "(no app_id)"))
        log("  Workspace ID: " .. tostring(win.workspace_id or "(none)"))
        log("  Is focused: " .. tostring(win.is_focused))
        log("  Is floating: " .. tostring(win.is_floating))
    end
else
    log("windows() returned nil")
end

-- Test 2: focused_window()
log("\n[Test 2] Testing niri.state.focused_window()")
local focused = niri.state.focused_window()
if focused then
    log("focused_window() succeeded")
    log("  ID: " .. tostring(focused.id))
    log("  Title: " .. (focused.title or "(no title)"))
    log("  App ID: " .. (focused.app_id or "(no app_id)"))
else
    log("focused_window() returned nil (no focused window)")
end

-- Test 3: workspaces()
log("\n[Test 3] Testing niri.state.workspaces()")
local workspaces = niri.state.workspaces()
if workspaces then
    log("workspaces() succeeded, got " .. #workspaces .. " workspaces")
    if #workspaces > 0 then
        log("First workspace:")
        local ws = workspaces[1]
        log("  ID: " .. tostring(ws.id))
        log("  Index: " .. tostring(ws.idx))
        log("  Name: " .. (ws.name or "(no name)"))
        log("  Output: " .. (ws.output or "(no output)"))
        log("  Is active: " .. tostring(ws.is_active))
        log("  Is focused: " .. tostring(ws.is_focused))
    end
else
    log("workspaces() returned nil")
end

-- Test 4: outputs()
log("\n[Test 4] Testing niri.state.outputs()")
local outputs = niri.state.outputs()
if outputs then
    log("outputs() succeeded, got " .. #outputs .. " outputs")
    if #outputs > 0 then
        log("First output:")
        local out = outputs[1]
        log("  Name: " .. out.name)
        log("  Make: " .. (out.make or "(unknown)"))
        log("  Model: " .. (out.model or "(unknown)"))
        if out.logical then
            log("  Width: " .. tostring(out.logical.width))
            log("  Height: " .. tostring(out.logical.height))
        end
    end
else
    log("outputs() returned nil")
end

-- Test 5: focused_output()
log("\n[Test 5] Testing niri.state.focused_output()")
local focused_out = niri.state.focused_output()
if focused_out then
    log("focused_output() succeeded")
    log("  Name: " .. focused_out.name)
else
    log("focused_output() returned nil (no focused output)")
end

-- Test 6: keyboard_layouts()
log("\n[Test 6] Testing niri.state.keyboard_layouts()")
local layouts = niri.state.keyboard_layouts()
if layouts then
    log("keyboard_layouts() succeeded")
    log("  Current index: " .. tostring(layouts.current_idx))
    if layouts.names then
        log("  Available layouts: " .. table.concat(layouts.names, ", "))
    end
else
    log("keyboard_layouts() returned nil")
end

log("\n=== Runtime State API Tests Complete ===")
