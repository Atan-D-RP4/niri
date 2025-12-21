-- Nested module to test dot notation require
local M = {}

M.depth = "nested"

function M.get_path()
    return "nested.deep_module"
end

return M
