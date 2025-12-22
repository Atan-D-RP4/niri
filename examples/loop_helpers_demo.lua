-- examples/loop_helpers_demo.lua
-- Demonstrates timer methods and wait() functionality
--
-- This example shows:
-- - Timer creation and lifecycle
-- - New timer methods: get_due_in(), set_repeat(), get_repeat(), timer.id
-- - The wait() function for sleeping and condition polling

niri.utils.log("=== Loop Helpers Demo ===")

-- Current monotonic time
local start_time = niri.loop.now()
niri.utils.log("Start time (ms): " .. start_time)

-- Create a repeating timer
local t = niri.loop.new_timer()
local tick_count = 0

t:start(100, 500, function()
    tick_count = tick_count + 1
    niri.utils.log("Tick #" .. tick_count)
    
    -- Stop after 3 ticks
    if tick_count >= 3 then
        t:close()
        niri.utils.log("Timer closed after 3 ticks")
    end
end)

-- Demonstrate timer properties
niri.utils.log("Timer ID: " .. t.id)
niri.utils.log("Is active: " .. tostring(t:is_active()))
niri.utils.log("Due in (ms): " .. t:get_due_in())
niri.utils.log("Repeat interval: " .. t:get_repeat())

-- Change the repeat interval
t:set_repeat(200)
niri.utils.log("New repeat interval: " .. t:get_repeat())

-- Demonstrate wait() without condition (simple sleep)
niri.utils.log("Sleeping for 100ms...")
local ok, _ = niri.loop.wait(100)
niri.utils.log("Sleep done, ok=" .. tostring(ok))

-- Demonstrate wait() with a condition
niri.utils.log("Waiting for condition (will succeed immediately)...")
local ok2, val = niri.loop.wait(1000, function()
    -- Return a truthy value to end wait early
    return { found = true, timestamp = niri.loop.now() }
end, 10)

if ok2 then
    niri.utils.log("Condition met! Found=" .. tostring(val.found))
else
    niri.utils.log("Timed out")
end

-- Demonstrate wait() timeout
niri.utils.log("Waiting for condition that never succeeds (100ms timeout)...")
local ok3, _ = niri.loop.wait(100, function()
    return false  -- Never succeeds
end, 20)

if ok3 then
    niri.utils.log("Unexpectedly succeeded")
else
    niri.utils.log("Timed out as expected")
end

-- Demonstrate Lua truthiness in wait()
-- Remember: 0, "", and {} are truthy in Lua!
local ok4, val4 = niri.loop.wait(100, function()
    return 0  -- 0 is truthy in Lua
end)
niri.utils.log("Zero is truthy: ok=" .. tostring(ok4) .. ", val=" .. tostring(val4))

local ok5, val5 = niri.loop.wait(100, function()
    return ""  -- Empty string is truthy in Lua
end)
niri.utils.log("Empty string is truthy: ok=" .. tostring(ok5) .. ", val=\"" .. val5 .. "\"")

-- One-shot timer that cleans up after itself
local oneshot = niri.loop.new_timer()
oneshot:start(50, 0, function()
    niri.utils.log("One-shot timer fired!")
    oneshot:close()  -- Always close one-shot timers
end)

local elapsed = niri.loop.now() - start_time
niri.utils.log("Demo completed in " .. elapsed .. "ms")
niri.utils.log("=== End Loop Helpers Demo ===")
