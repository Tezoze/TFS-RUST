local creatureevent = CreatureEvent("PlayerDeath")

function creatureevent.onDeath(player)
	local playerId = player:getId()
	if nextUseStaminaTime[playerId] then
		nextUseStaminaTime[playerId] = nil
	end

	player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "You are dead.")
end

creatureevent:register()