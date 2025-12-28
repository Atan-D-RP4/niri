-- format_value.lua - Pretty-print any Lua value (like vim.print)
local function format_value(val, indent, seen, compact_limit, in_table)
	indent = indent or 0
	seen = seen or {}
	compact_limit = compact_limit or 80
	in_table = in_table or false

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
		-- Only quote strings when inside tables for clarity
		if in_table then
			return string.format("%q", val)
		else
			return val
		end
	elseif type(val) == "function" then
		return "<function>"
	elseif type(val) == "userdata" then
		local mt = getmetatable(val)
		-- Try inspect() method first (returns table of properties)
		if type(val.inspect) == "function" then
			local ok, props = pcall(val.inspect, val)
			if ok and type(props) == "table" then
				if seen[val] then
					return "{ ... }"
				end
				seen[val] = true

				local items = {}
				local keys = {}
				for k in pairs(props) do
					table.insert(keys, k)
				end
				table.sort(keys, function(a, b)
					return tostring(a) < tostring(b)
				end)

				-- Extract type name from __tostring if available
				local type_name = mt and mt.__tostring and tostring(val):match("^(%w+)") or ""
				for _, k in ipairs(keys) do
					local v = props[k]
					local key_str = tostring(k)
					local val_str
					if type(v) == "string" and v:match("^<method:") then
						val_str = v
					else
						val_str = format_value(v, indent + 1, seen, compact_limit)
					end
					table.insert(items, key_str .. " = " .. val_str)
				end

				if #items == 0 then
					return type_name .. type_name ~= "" and " " or "" .. "{}"
				end

				local single_line = type_name .. " { " .. table.concat(items, ", ") .. " }"
				if #single_line <= compact_limit then
					return single_line
				else
					return type_name .. type_name ~= "" and " "
						or ""
							.. "{\n"
							.. indstr_next
							.. table.concat(items, ",\n" .. indstr_next)
							.. "\n"
							.. indstr
							.. "}"
				end
			end
		end
		-- Fall back to __tostring
		if mt and mt.__tostring then
			return tostring(val)
		end
		return "<userdata>"
	elseif type(val) == "thread" then
		return "<thread>"
	elseif type(val) == "table" then
		if seen[val] then
			return "{ ... }"
		end
		seen[val] = true

		local items = {}
		local is_array = true
		local max_index = 0

		for k, _ in pairs(val) do
			if type(k) == "number" and k > 0 and k == math.floor(k) then
				max_index = math.max(max_index, k)
			else
				is_array = false
			end
		end

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
			for i = 1, max_index do
				table.insert(items, format_value(val[i], indent + 1, seen, compact_limit, true))
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
			local keys = {}
			for k in pairs(val) do
				table.insert(keys, k)
			end
			table.sort(keys, function(a, b)
				local ta, tb = type(a), type(b)
				if ta ~= tb then
					if ta == "number" then
						return true
					end
					if tb == "number" then
						return false
					end
				end
				return tostring(a) < tostring(b)
			end)

			for _, k in ipairs(keys) do
				local v = val[k]
				local key_str
				if type(k) == "string" and k:match("^[a-zA-Z_][a-zA-Z0-9_]*$") then
					key_str = k
				else
					key_str = "[" .. format_value(k, 0, seen, compact_limit, true) .. "]"
				end
				table.insert(items, key_str .. " = " .. format_value(v, indent + 1, seen, compact_limit, true))
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
	local args = { ... }
	local parts = {}
	for _, val in ipairs(args) do
		table.insert(parts, format_value(val))
	end
	print(table.concat(parts, "\t"))
end

return {
	format_value = format_value,
	nice_print = nice_print,
}
