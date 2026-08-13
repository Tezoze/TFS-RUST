-- Port of `createFunctions` from `data/lib/compat/compat.lua`.
-- Gap 7c: `data/scripts/lib/create_functions.lua` calls this; resolved
-- decision #3 does not load the full compat layer, so the helper lives here.
-- C++/TFS domain: generates getX/setX aliases on a class table for methods
-- whose names do not already start with is/get/set/add/can/need.

function createFunctions(class)
	local exclude = {[2] = {"is"}, [3] = {"get", "set", "add", "can"}, [4] = {"need"}}
	local temp = {}
	for name, func in pairs(class) do
		local add = true
		for strLen, strTable in pairs(exclude) do
			if table.contains(strTable, name:sub(1, strLen)) then
				add = false
			end
		end
		if add then
			local str = name:sub(1, 1):upper() .. name:sub(2)
			local getFunc = function(self) return func(self) end
			local setFunc = function(self, ...) return func(self, ...) end
			local get = "get" .. str
			local set = "set" .. str
			if not (rawget(class, get) and rawget(class, set)) then
				table.insert(temp, {set, setFunc, get, getFunc})
			end
		end
	end
	for _, func in ipairs(temp) do
		rawset(class, func[1], func[2])
		rawset(class, func[3], func[4])
	end
end
