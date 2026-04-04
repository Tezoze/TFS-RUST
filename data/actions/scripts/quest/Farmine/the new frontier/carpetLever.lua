local config = {
	[9119] = Position(32991, 31539, 1),
	[9118] = Position(32991, 31539, 4)
}

function onUse(player, item, fromPosition, target, toPosition, isHotkey)
	local targetPosition = config[item.actionid]
	if not targetPosition then
		return true
	end

	local wasOff = item.itemid == 1945
	item:transform(wasOff and 1946 or 1945)

	toPosition.x = (item.actionid == 8007) and toPosition.x + 1 or toPosition.x - 1
	local creature = Tile(toPosition):getTopCreature()
	if not creature or not creature:isPlayer() then
		player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "Stand in the correct position next to the lever!")
		return true
	end

	creature:teleportTo(targetPosition)
	return true
end
