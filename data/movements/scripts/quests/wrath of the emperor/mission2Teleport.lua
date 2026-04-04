function onStepIn(creature, item, position, fromPosition)
	local player = creature:getPlayer()
	if not player then
		return true
	end

	-- Check if Wrath of the Emperor Mission 02 is completed
	if player:getStorageValue(Storage.WrathoftheEmperor.Mission02) < 2 then
		player:teleportTo(fromPosition, false)
		player:getPosition():sendMagicEffect(CONST_ME_TELEPORT)
		return true
	end

	-- Mission 02 is completed - allow teleport
	-- SET YOUR DESTINATION HERE
	local destination = Position(33211, 31065, 9) -- Replace with actual coordinates

	player:getPosition():sendMagicEffect(CONST_ME_TELEPORT)
	player:teleportTo(destination)
	destination:sendMagicEffect(CONST_ME_TELEPORT)

	return true
end
