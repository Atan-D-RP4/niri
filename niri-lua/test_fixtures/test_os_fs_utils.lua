-- Integration tests for niri.os and niri.fs utilities
-- These tests run in the actual Luau environment with safe libraries only

-- Test niri.os.hostname()
local hostname = niri.os.hostname()
assert(type(hostname) == "string", "hostname should return string")
assert(#hostname > 0, "hostname should not be empty")

-- Test niri.os.getenv()
-- HOME is typically set on Unix systems
local home = niri.os.getenv("HOME")
assert(home == nil or type(home) == "string", "getenv should return string or nil")

-- Test getenv with non-existent variable
local nonexistent = niri.os.getenv("__NIRI_TEST_NONEXISTENT_VAR_12345__")
assert(nonexistent == nil, "getenv should return nil for unset variables")

-- Test niri.fs.readable()
-- Test with a path that definitely doesn't exist
assert(niri.fs.readable("/nonexistent/path/12345/file.txt") == false, "nonexistent path not readable")

-- Test with empty path
assert(niri.fs.readable("") == false, "empty path not readable")

-- Test niri.fs.expand()
-- Test with a simple path (no expansion needed)
local simple = niri.fs.expand("/usr/bin")
assert(simple == "/usr/bin", "expand should return unchanged path when no expansion needed")

-- Test tilde expansion (if HOME is set)
local home_env = niri.os.getenv("HOME")
if home_env then
    local expanded = niri.fs.expand("~")
    assert(expanded == home_env, "~ should expand to HOME: got " .. tostring(expanded) .. " expected " .. tostring(home_env))
    
    local expanded_config = niri.fs.expand("~/.config")
    assert(expanded_config == home_env .. "/.config", "~/.config should expand correctly")
end

-- Test niri.fs.which()
-- sh should exist on all Unix systems
local sh = niri.fs.which("sh")
assert(sh ~= nil, "sh should be found in PATH")
assert(type(sh) == "string", "which should return string")
assert(#sh > 0, "which result should not be empty")

-- Test with non-existent command
local missing = niri.fs.which("__nonexistent_command_12345__")
assert(missing == nil, "missing command should return nil")

-- Test with empty string
local empty = niri.fs.which("")
assert(empty == nil, "empty command should return nil")

-- Primary use case test: conditional feature enablement
-- This is the main use case from the spec
local has_xwayland = niri.fs.which("xwayland-satellite")
assert(has_xwayland == nil or type(has_xwayland) == "string", "which returns string or nil")

-- Success message
niri.utils.log("niri.os and niri.fs integration tests passed!")
