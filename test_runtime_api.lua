-- Test script for Tier 3 Runtime API
-- This script tests the niri.runtime API functions

print("=== Testing Tier 3 Runtime API ===")

-- Test 1: get_windows()
print("\n[Test 1] Testing niri.runtime.get_windows()")
local windows = niri.runtime.get_windows()
if windows then
    print("✓ get_windows() succeeded, got", #windows, "windows")
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
    print("✗ get_windows() returned nil")
end

-- Test 2: get_focused_window()
print("\n[Test 2] Testing niri.runtime.get_focused_window()")
local focused = niri.runtime.get_focused_window()
if focused then
    print("✓ get_focused_window() succeeded")
    print("  ID:", focused.id)
    print("  Title:", focused.title or "(no title)")
    print("  App ID:", focused.app_id or "(no app_id)")
else
    print("✓ get_focused_window() returned nil (no focused window)")
end

-- Test 3: get_workspaces()
print("\n[Test 3] Testing niri.runtime.get_workspaces()")
local workspaces = niri.runtime.get_workspaces()
if workspaces then
    print("✓ get_workspaces() succeeded, got", #workspaces, "workspaces")
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
    print("✗ get_workspaces() returned nil")
end

-- Test 4: get_outputs()
print("\n[Test 4] Testing niri.runtime.get_outputs()")
local outputs = niri.runtime.get_outputs()
if outputs then
    print("✓ get_outputs() succeeded, got", #outputs, "outputs")
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
    print("✗ get_outputs() returned nil")
end

print("\n=== Tier 3 Runtime API Tests Complete ===")
