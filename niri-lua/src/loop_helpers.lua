--- Execute a function after a delay (one-shot timer)
--- @param fn function Callback to execute
--- @param delay_ms number Delay in milliseconds
--- @return Timer Timer handle (can be stopped before firing)
function niri.loop.defer(fn, delay_ms)
    assert(type(fn) == "function", "defer: fn must be a function")
    assert(
        type(delay_ms) == "number" and delay_ms >= 0 and delay_ms == delay_ms and delay_ms ~= math.huge,
        "defer: delay_ms must be finite non-negative number"
    )

    local timer = niri.loop.new_timer()
    timer:start(delay_ms, 0, function()
        timer:close()
        fn()
    end)
    return timer
end

--- Wrap a function to auto-schedule on main event loop
--- @param fn function Function to wrap
--- @return function Wrapped function that schedules original
function niri.schedule_wrap(fn)
    assert(type(fn) == "function", "schedule_wrap: fn must be a function")
    return function(...)
        local args = {...}
        niri.schedule(function()
            fn(table.unpack(args))
        end)
    end
end
