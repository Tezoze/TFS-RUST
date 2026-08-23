local creatureevent = CreatureEvent("PlayerDeath")

function creatureevent.onDeath(player)
	player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "You are dead.")
end

creatureevent:register()
