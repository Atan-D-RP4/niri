-- niri.state.watch implementation
niri.state = niri.state or {}

local active_subscriptions = setmetatable({}, { __mode = "v" })

function niri.state.watch(opts, callback)
    assert(type(opts) == "table", "watch: opts must be a table")
    assert(type(callback) == "function", "watch: callback must be a function")
    assert(opts.events and #opts.events > 0, "watch: events list required")

    local events = opts.events
    local filter = opts.filter
    local immediate = opts.immediate
    local debounce_ms = opts.debounce_ms

    local active = true
    local timer = nil
    local last_payload = nil
    local handler_ids = {}

    -- Debounced callback wrapper
    local function invoke(payload)
        if not active then return end
        if filter and not filter(payload) then return end

        if debounce_ms and debounce_ms > 0 then
            last_payload = payload
            if timer then timer:stop() end
            timer = niri.loop.new_timer()
            timer:start(debounce_ms, 0, function()
                timer:close()
                if active and last_payload then
                    callback(last_payload)
                    last_payload = nil
                end
            end)
        else
            callback(payload)
        end
    end

    -- Subscribe to events
    for _, event_name in ipairs(events) do
        local id = niri.events:on(event_name, invoke)
        table.insert(handler_ids, { event = event_name, id = id })
    end

    -- Immediate delivery
    if immediate then
        niri.loop.defer(function()
            if active then
                callback({ immediate = true })
            end
        end, 0)
    end

    -- Subscription object
    local sub = {
        cancel = function()
            if not active then return end
            active = false
            for _, h in ipairs(handler_ids) do
                niri.events:off(h.event, h.id)
            end
            if timer then
                timer:stop()
                timer:close()
            end
            handler_ids = {}
            timer = nil
            last_payload = nil
        end,
        is_active = function()
            return active
        end,
    }

    -- Track for potential GC cleanup
    active_subscriptions[sub] = sub

    -- GC cleanup via __gc proxy
    local gc_guard = setmetatable({}, {
        __gc = function()
            if active then
                sub.cancel()
            end
        end,
    })
    rawset(sub, "__gc_guard", gc_guard)

    return sub
end
