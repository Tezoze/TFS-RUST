local liquidContainers = {
	2873, 2524
}

local millstones = {
	1943, 1944, 1945, 1946
}

local ovens = {
	2535, 2537, 2539, 2541,
}

local action = Action()

function action.onUse(player, item, fromPosition, target, toPosition)
	local itemId = item:getId()
	if itemId == 3603 then
		if target.type == FLUID_WATER and table.contains(liquidContainers, target.itemid) then
			item:remove(1)
			player:addItem(3604, 1)
			target:transform(target.itemid, FLUID_NONE)
			return true
		end
	elseif table.contains(millstones, target.itemid) and item.itemid ~= 3604 then
		item:remove(1)
		player:addItem(3603, 1)
		return true
	elseif table.contains(ovens, target.itemid) then
		if itemId == 3604 then
			item:remove(1)
			Game.createItem(3600, 1, toPosition)
			return true
		end
	end
	return false
end

action:id(3603)
action:id(3605)
action:id(3604)
action:register()
