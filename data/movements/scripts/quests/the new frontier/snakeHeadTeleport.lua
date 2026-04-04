function onStepIn(creature, item, position, fromPosition)
	local player = creature:getPlayer()
	if not player then
		return true
	end

	player:getPosition():sendMagicEffect(CONST_ME_TELEPORT)

	if player:getStorageValue(Storage.TheNewFrontier.TomeofKnowledge) >= 7 then
		-- Teleport to northern Zao teleporter (one-way teleport)
		player:teleportTo(Position(33197, 31346, 6))
	else
		player:teleportTo(fromPosition)
	end

	player:getPosition():sendMagicEffect(CONST_ME_TELEPORT)

	return true
end
