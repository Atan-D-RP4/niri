-- Module that tracks load count to test caching
-- Each load increments the global counter
_G.counter_load_count = (_G.counter_load_count or 0) + 1

local M = {}

function M.get_load_count()
    return _G.counter_load_count
end

return M
