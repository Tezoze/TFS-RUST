-- Cuckoo 1873–1876 are InformationType=2 + Expire (`objects.srv`); use announces
-- time via watch.lua. Do not toggle them here.
local decayItems = {
	[2041] = 2042, [2042] = 2041, -- candelabrum
	[2044] = 2045, [2045] = 2044, -- lamp
	[2047] = 2048, [2048] = 2047, -- candlestick
	[2050] = 2051, [2051] = 2050, -- torch
	[2052] = 2053, [2053] = 2052, -- torch
	[2054] = 2055, [2055] = 2054, -- torch
	[2162] = 2163, [2163] = 2162, -- magic light wand
	[2057] = 2041, -- eternal candelabrum into expiring candelabrum
}

local action = Action()

function action.onUse(player, item, fromPosition, target, toPosition)
	local transformIds = decayItems[item:getId()]
	if not transformIds then
		return false
	end

	item:transform(transformIds)
	item:decay()
	return true
end

for id in pairs(decayItems) do
	action:id(id)
end

action:register()
