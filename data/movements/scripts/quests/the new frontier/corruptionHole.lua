function onStepIn(creature, item, position, fromPosition)
	local player = creature:getPlayer()
	if not player then
		return true
	end

	-- Check if the hole is open (item ID 7933)
	if item.itemid ~= 7933 then
		return true -- Allow stepping on closed hole without teleport
	end

	-- Check if player has completed the 8th Tome of Knowledge (Corruption Hole access)
	if player:getStorageValue(Storage.TheNewFrontier.TomeofKnowledge) < 8 then
		player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "It looks dangerous to enter this hole, you should talk with Cael to find out more about it.")
		player:teleportTo(fromPosition, true)
		return false
	end

	-- Allow entry if requirement is met and hole is open
	local destination = Position(33344, 31115, 8)
	player:teleportTo(destination)
	return true
end
