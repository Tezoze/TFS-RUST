function onStepIn(creature, item, position, fromPosition)
	local player = creature:getPlayer()
	if not player then
		return true
	end

	-- Add quest-specific logic here if needed
	-- For now, just allow the player to enter

	player:teleportTo(position)
	player:getPosition():sendMagicEffect(CONST_ME_TELEPORT)
	return true
end
