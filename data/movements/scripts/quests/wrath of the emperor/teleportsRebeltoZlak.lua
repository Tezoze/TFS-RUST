local config = {
	[12382] = {storageKey = Storage.WrathoftheEmperor.Mission05, toPosition = {Position(33078, 31219, 8), Position(33216, 31069, 9)}},
	[12383] = {storageKey = Storage.WrathoftheEmperor.Mission05, toPosition = {Position(33216, 31069, 9), Position(33078, 31219, 8)}}
}

function onStepIn(creature, item, position, fromPosition)
	local player = creature:getPlayer()
	if not player then
		return true
	end

	local targetTile = config[item.actionid]
	if not targetTile then
		return true
	end

	local hasStorageValue = player:getStorageValue(targetTile.storageKey) >= 1
	player:getPosition():sendMagicEffect(CONST_ME_TELEPORT)

	if hasStorageValue then
		player:teleportTo(targetTile.toPosition[1])
	else
		player:teleportTo(fromPosition)
		player:say('This portal is not activated', TALKTYPE_MONSTER_SAY)
	end

	player:getPosition():sendMagicEffect(CONST_ME_TELEPORT)
	
	return true
end
