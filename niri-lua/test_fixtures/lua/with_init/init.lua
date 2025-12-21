-- Module using init.lua convention
local M = {}

M.type = "init_module"

function M.describe()
    return "Loaded via init.lua"
end

return M
