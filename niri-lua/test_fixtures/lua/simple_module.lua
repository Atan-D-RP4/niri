-- Simple test module that returns a table
local M = {}

M.name = "simple_module"
M.version = "1.0"

function M.greet(name)
    return "Hello, " .. name .. "!"
end

return M
