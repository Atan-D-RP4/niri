-- Test file that uses relative require
-- This file should be loaded first, then it requires ./lua/simple_module
local simple = require("./lua/simple_module")

return {
    loaded_module = simple,
    greeting = simple.greet("World")
}
