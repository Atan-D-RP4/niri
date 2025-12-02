-- Test script for Runtime State API
-- This script tests the niri.state API functions

print("=== Testing Runtime State API ===")

-- Test 1: windows()
print("\n[Test 1] Testing niri.state.windows()")
local windows = niri.state.windows()
if windows then
    print("✓ windows() succeeded, got", #windows, "windows")
    if #windows > 0 then
        print("First window:")
        local win = windows[1]
        print("  ID:", win.id)
        print("  Title:", win.title or "(no title)")
        print("  App ID:", win.app_id or "(no app_id)")
        print("  Workspace ID:", win.workspace_id or "(none)")
        print("  Is focused:", win.is_focused)
        print("  Is floating:", win.is_floating)
    end
else
    print("✗ windows() returned nil")
end

-- Test 2: focused_window()
print("\n[Test 2] Testing niri.state.focused_window()")
local focused = niri.state.focused_window()
if focused then
    print("✓ focused_window() succeeded")
    print("  ID:", focused.id)
    print("  Title:", focused.title or "(no title)")
    print("  App ID:", focused.app_id or "(no app_id)")
else
    print("✓ focused_window() returned nil (no focused window)")
end

-- Test 3: workspaces()
print("\n[Test 3] Testing niri.state.workspaces()")
local workspaces = niri.state.workspaces()
if workspaces then
    print("✓ workspaces() succeeded, got", #workspaces, "workspaces")
    if #workspaces > 0 then
        print("First workspace:")
        local ws = workspaces[1]
        print("  ID:", ws.id)
        print("  Index:", ws.idx)
        print("  Name:", ws.name or "(no name)")
        print("  Output:", ws.output or "(no output)")
        print("  Is active:", ws.is_active)
        print("  Is focused:", ws.is_focused)
    end
else
    print("✗ workspaces() returned nil")
end

-- Test 4: outputs()
print("\n[Test 4] Testing niri.state.outputs()")
local outputs = niri.state.outputs()
if outputs then
    print("✓ outputs() succeeded, got", #outputs, "outputs")
    if #outputs > 0 then
        print("First output:")
        local out = outputs[1]
        print("  Name:", out.name)
        print("  Make:", out.make or "(unknown)")
        print("  Model:", out.model or "(unknown)")
        print("  Width:", out.logical and out.logical.width or "(unknown)")
        print("  Height:", out.logical and out.logical.height or "(unknown)")
    end
else
    print("✗ outputs() returned nil")
end

print("\n=== Runtime State API Tests Complete ===")
