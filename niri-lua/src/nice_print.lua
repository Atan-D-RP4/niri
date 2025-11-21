-- Nice print function for tables and values (similar to vim.print)
local function format_value(val, indent, seen, compact_limit)
    indent = indent or 0
    seen = seen or {}
    compact_limit = compact_limit or 60

    local indstr = string.rep("  ", indent)
    local indstr_next = string.rep("  ", indent + 1)

    if val == nil then
        return "nil"
    elseif type(val) == "boolean" then
        return tostring(val)
    elseif type(val) == "number" then
        if val == math.floor(val) then
            return string.format("%.0f", val)
        else
            return tostring(val)
        end
    elseif type(val) == "string" then
        return string.format("%q", val)
    elseif type(val) == "table" then
        if seen[val] then
            return "{ ... }"
        end
        seen[val] = true

        local items = {}
        local is_array = true
        local max_index = 0

        -- Check if array-like
        for k, _ in pairs(val) do
            if type(k) == "number" and k > 0 and k == math.floor(k) then
                max_index = math.max(max_index, k)
            else
                is_array = false
            end
        end

        -- Verify array is continuous
        if is_array and max_index > 0 then
            for i = 1, max_index do
                if val[i] == nil then
                    is_array = false
                    break
                end
            end
        else
            is_array = false
        end

        if is_array and max_index > 0 then
            -- Format as array
            for i = 1, max_index do
                table.insert(items, format_value(val[i], indent + 1, seen, compact_limit))
            end

            if #items == 0 then
                return "{}"
            end

            local single_line = "{ " .. table.concat(items, ", ") .. " }"
            if #single_line <= compact_limit then
                return single_line
            else
                return "{\n" .. indstr_next .. table.concat(items, ",\n" .. indstr_next) .. "\n" .. indstr .. "}"
            end
        else
            -- Format as object
            local keys = {}
            for k in pairs(val) do
                table.insert(keys, k)
            end
            table.sort(keys, function(a, b)
                local ta, tb = type(a), type(b)
                if ta ~= tb then
                    if ta == "number" then return true end
                    if tb == "number" then return false end
                end
                return tostring(a) < tostring(b)
            end)

            for _, k in ipairs(keys) do
                local v = val[k]
                local key_str
                if type(k) == "string" and k:match("^[a-zA-Z_][a-zA-Z0-9_]*$") then
                    key_str = k
                else
                    key_str = "[" .. format_value(k, 0, seen, compact_limit) .. "]"
                end
                table.insert(items, key_str .. " = " .. format_value(v, indent + 1, seen, compact_limit))
            end

            if #items == 0 then
                return "{}"
            end

            local single_line = "{ " .. table.concat(items, ", ") .. " }"
            if #single_line <= compact_limit then
                return single_line
            else
                return "{\n" .. indstr_next .. table.concat(items, ",\n" .. indstr_next) .. "\n" .. indstr .. "}"
            end
        end
    else
        return string.format("<%s>", type(val))
    end
end

local function nice_print(...)
    local args = {...}
    for i, val in ipairs(args) do
        if i > 1 then
            io.write("\t")
        end
        io.write(format_value(val))
    end
    io.write("\n")
end

return nice_print
