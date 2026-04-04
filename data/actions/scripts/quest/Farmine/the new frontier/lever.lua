local config = {
	[8005] = Position(33055, 31527, 14),
	[8006] = Position(33065, 31489, 15),
	[9120] = Position(33061, 31527, 12),
	[9121] = Position(32993, 31547, 4)
}

function onUse(player, item, fromPosition, target, toPosition, isHotkey)
	local targetPosition = config[item.actionid]
	if not targetPosition then
		return true
	end

	local wasOff = item.itemid == 1945
	item:transform(wasOff and 1946 or 1945)

	toPosition.x = (item.actionid == 8005) and toPosition.x + 1 or toPosition.x - 1
	local creature = Tile(toPosition):getTopCreature()
	if not creature or not creature:isPlayer() then
		player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "Stand in the correct position next to the lever!")
		return true
	end

	if item.actionid == 8005 or item.actionid == 9120 then
		local mission05 = creature:getStorageValue(Storage.TheNewFrontier.Mission05)
		local mission03 = creature:getStorageValue(Storage.TheNewFrontier.Mission03)
		if mission05 == 7 then -- Exact match like Brodrosch
			targetPosition.z = 10
		elseif mission03 >= 2 then -- Match Brodrosch logic
			targetPosition.z = 12
		else
			targetPosition.z = 14
		end
	end
	creature:teleportTo(targetPosition)
	return true
end
