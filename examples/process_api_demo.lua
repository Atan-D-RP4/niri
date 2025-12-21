#!/usr/bin/env niri
-- Process API Demo
--
-- This example demonstrates the Process API for spawning and managing
-- external processes from Lua scripts.
--
-- The Process API provides:
--   - Fire-and-forget spawning: niri.action:spawn({"cmd"})
--   - Managed spawning with opts: niri.action:spawn({"cmd"}, opts)
--   - ProcessHandle for process control (wait, kill, write, etc.)
--   - Streaming callbacks for stdout/stderr
--   - Exit callbacks with result code/signal
--
-- SpawnOpts table fields:
--   cwd            - Working directory for the process
--   env            - Environment variables (table)
--   clear_env      - Clear inherited environment (boolean)
--   stdin          - "closed", "pipe", or string data
--   capture_stdout - Buffer stdout for result.stdout (boolean)
--   capture_stderr - Buffer stderr for result.stderr (boolean)
--   stdout         - Streaming callback: function(err, data)
--   stderr         - Streaming callback: function(err, data)
--   on_exit        - Exit callback: function(result, err)
--   text           - Line-buffered (true) or binary chunks (false)
--   detach         - Let process outlive Lua runtime (boolean)
--
-- ProcessHandle methods:
--   handle.pid        - Process ID (read-only field)
--   handle:wait(ms)   - Wait for exit with optional timeout
--   handle:kill(sig)  - Send signal (default: SIGKILL)
--   handle:write(data)- Write to stdin (requires stdin="pipe")
--   handle:close_stdin() - Close stdin pipe
--   handle:is_closing()  - Check if stdin is closed

-- Apply minimal config
niri.config:apply()

niri.utils.log("=== Niri Process API Demo ===")

-- =============================================================================
-- Example 1: Fire-and-forget spawning
-- =============================================================================
niri.utils.log("")
niri.utils.log("--- Example 1: Fire-and-forget ---")

-- Simple spawn without options - returns nil, no way to track
niri.action:spawn({"notify-send", "Niri", "Fire-and-forget spawn!"})
niri.utils.log("Sent notification (fire-and-forget)")

-- =============================================================================
-- Example 2: Managed spawn with ProcessHandle
-- =============================================================================
niri.utils.log("")
niri.utils.log("--- Example 2: Managed spawn with wait() ---")

-- Spawn with empty opts to get a handle
local handle = niri.action:spawn({"echo", "Hello from managed process"}, {})
if handle then
    niri.utils.log("Spawned process with PID: " .. handle.pid)
    local result = handle:wait()
    niri.utils.log("Process exited with code: " .. (result.code or "nil"))
end

-- =============================================================================
-- Example 3: Capturing stdout
-- =============================================================================
niri.utils.log("")
niri.utils.log("--- Example 3: Capturing stdout ---")

local handle = niri.action:spawn({"hostname"}, {capture_stdout = true})
if handle then
    local result = handle:wait()
    -- Note: captured stdout is available via streaming callbacks or result.stdout
    -- In synchronous wait(), stdout may need event processing
    niri.utils.log("hostname exited with code: " .. (result.code or "nil"))
end

-- =============================================================================
-- Example 4: Streaming callbacks (recommended for real-time output)
-- =============================================================================
niri.utils.log("")
niri.utils.log("--- Example 4: Streaming stdout callback ---")

-- Callback signature: function(err, data)
-- err is nil on success, data contains the output line/chunk
niri.action:spawn(
    {"sh", "-c", "echo 'Line 1'; sleep 0.1; echo 'Line 2'; sleep 0.1; echo 'Line 3'"},
    {
        stdout = function(_, data)
            if data then
                niri.utils.log("STDOUT: " .. data:gsub("\n", ""))
            end
        end,
        on_exit = function(result)
            niri.utils.log("Streaming example exited with code: " .. (result.code or "nil"))
        end
    }
)

-- =============================================================================
-- Example 5: Environment variables
-- =============================================================================
niri.utils.log("")
niri.utils.log("--- Example 5: Custom environment ---")

niri.action:spawn(
    {"sh", "-c", "echo $MY_CUSTOM_VAR"},
    {
        env = {MY_CUSTOM_VAR = "Hello from custom env!"},
        stdout = function(_, data)
            if data then
                niri.utils.log("ENV output: " .. data:gsub("\n", ""))
            end
        end
    }
)

-- =============================================================================
-- Example 6: Working directory
-- =============================================================================
niri.utils.log("")
niri.utils.log("--- Example 6: Custom working directory ---")

niri.action:spawn(
    {"pwd"},
    {
        cwd = "/tmp",
        stdout = function(_, data)
            if data then
                niri.utils.log("CWD is: " .. data:gsub("\n", ""))
            end
        end
    }
)

-- =============================================================================
-- Example 7: Stdin pipe for interactive processes
-- =============================================================================
niri.utils.log("")
niri.utils.log("--- Example 7: Writing to stdin ---")

local cat_handle = niri.action:spawn(
    {"cat"},
    {
        stdin = "pipe",
        stdout = function(_, data)
            if data then
                niri.utils.log("cat echoed: " .. data:gsub("\n", ""))
            end
        end,
        on_exit = function()
            niri.utils.log("cat exited")
        end
    }
)

if cat_handle then
    cat_handle:write("Hello from stdin!\n")
    cat_handle:write("Second line\n")
    cat_handle:close_stdin()  -- Signal EOF to cat
end

-- =============================================================================
-- Example 8: Stdin with immediate data (no pipe needed)
-- =============================================================================
niri.utils.log("")
niri.utils.log("--- Example 8: Stdin data (string) ---")

niri.action:spawn(
    {"wc", "-w"},  -- Count words
    {
        stdin = "one two three four five",
        stdout = function(_, data)
            if data then
                niri.utils.log("Word count: " .. data:gsub("%s+", ""))
            end
        end
    }
)

-- =============================================================================
-- Example 9: Process timeout with kill
-- =============================================================================
niri.utils.log("")
niri.utils.log("--- Example 9: Timeout handling ---")

local handle = niri.action:spawn({"sleep", "10"}, {})
if handle then
    niri.utils.log("Started sleep process (PID: " .. handle.pid .. ")")
    
    -- Wait with 100ms timeout - will timeout and escalate to SIGTERM then SIGKILL
    local result = handle:wait(100)
    
    if result.signal then
        niri.utils.log("Process killed with signal: " .. result.signal)
    else
        niri.utils.log("Process exited with code: " .. (result.code or "nil"))
    end
end

-- =============================================================================
-- Example 10: Stderr handling
-- =============================================================================
niri.utils.log("")
niri.utils.log("--- Example 10: Stderr callback ---")

niri.action:spawn(
    {"sh", "-c", "echo 'stdout message'; echo 'stderr message' >&2"},
    {
        stdout = function(_, data)
            if data then
                niri.utils.log("STDOUT: " .. data:gsub("\n", ""))
            end
        end,
        stderr = function(_, data)
            if data then
                niri.utils.log("STDERR: " .. data:gsub("\n", ""))
            end
        end
    }
)

-- =============================================================================
-- Example 11: Practical use case - Build notification
-- =============================================================================
niri.utils.log("")
niri.utils.log("--- Example 11: Practical - Build notification ---")

-- Simulate a build process with success notification
local function run_build(command, name)
    niri.action:spawn(
        {"sh", "-c", command},
        {
            on_exit = function(result)
                if result.code == 0 then
                    niri.action:spawn({"notify-send", "-i", "dialog-ok", name, "Build succeeded!"})
                else
                    niri.action:spawn({"notify-send", "-i", "dialog-error", name, "Build failed with code " .. (result.code or "?")})
                end
            end
        }
    )
end

-- Example: run a quick "build"
run_build("sleep 0.2 && true", "Demo Build")  -- Will succeed
-- run_build("sleep 0.2 && false", "Failing Build")  -- Would fail

-- =============================================================================
-- Example 12: Detached process
-- =============================================================================
niri.utils.log("")
niri.utils.log("--- Example 12: Detached process ---")

-- Detached processes continue running even after Lua GC collects the handle
local handle = niri.action:spawn(
    {"sleep", "1"},
    {detach = true}
)
if handle then
    niri.utils.log("Started detached process (PID: " .. handle.pid .. ")")
    niri.utils.log("This process will continue even if we don't wait for it")
    -- Don't call wait() - let it run independently
end

niri.utils.log("")
niri.utils.log("=== Process API Demo Complete ===")
niri.utils.log("Check your notifications and logs for output.")
