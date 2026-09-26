-- Cuckoo 1873–1876 are InformationType=2 + Expire (`objects.srv`); use announces
-- time via watch.lua. Do not toggle them here.
local decayItems = {
	[2911] = 2912, [2912] = 2911, -- candelabrum
	[2914] = 2915, [2915] = 2914, -- lamp
	[2917] = 2918, [2918] = 2917, -- candlestick
	[2920] = 2921, [2921] = 2920, -- torch
	[2922] = 2923, [2923] = 2922, -- torch
	[2924] = 2925, [2925] = 2924, -- torch
	[3046] = 3047, [3047] = 3046, -- magic light wand
	[2927] = 2911, -- eternal candelabrum into expiring candelabrum
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
