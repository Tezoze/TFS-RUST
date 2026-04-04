local config = {
	[50084] = Position(32857, 32667, 9),
	[50085] = Position(32892, 32632, 11),
	[50086] = Position(32886, 32632, 11)
}

function onStepIn(creature, item, position, fromPosition)
	local player = creature:getPlayer()
	if not player then
		return true
	end

	local targetPosition = config[item.actionid]
	if not targetPosition then
		return true
	end

	local storageKey = 30276 -- Use direct storage ID to avoid potential nil issues
	local storageValue = player:getStorageValue(storageKey)
	-- Allow teleport if storage value is nil, -1 (not set), or less than 100
	if not storageValue or storageValue == -1 or storageValue < 100 then
		player:teleportTo(targetPosition)
		player:getPosition():sendMagicEffect(CONST_ME_TELEPORT)
	end
	return true
end
