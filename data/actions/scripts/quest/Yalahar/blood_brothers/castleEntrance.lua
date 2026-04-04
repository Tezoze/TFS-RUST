-- Blood Brothers Quest - Castle Entrance
-- Mission 4: The Dark Lands - Entry to Vengoth Castle

function onUse(player, item, fromPosition, target, toPosition, isHotkey)
	if player:getStorageValue(Storage.BloodBrothers.Mission04) == 1 then
		-- First time entering castle - complete Mission 4
		player:setStorageValue(Storage.BloodBrothers.Mission04, 2)
		player:addExperience(1000, true)
		player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "You have discovered the vampire castle! The air is thick with evil.")
		player:teleportTo(Position(32858, 31549, 8)) -- Inside castle entrance
		player:getPosition():sendMagicEffect(CONST_ME_TELEPORT)
	elseif player:getStorageValue(Storage.BloodBrothers.VengothAccess) == 1 then
		-- Already have access
		player:teleportTo(Position(32858, 31549, 8)) -- Inside castle entrance
		player:getPosition():sendMagicEffect(CONST_ME_TELEPORT)
	else
		player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "You have no business here.")
	end
	
	return true
end
