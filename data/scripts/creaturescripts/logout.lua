local creatureevent = CreatureEvent("PlayerLogout")

function creatureevent.onLogout(player)
	local playerId = player:getId()
	if nextUseStaminaTime[playerId] then
		nextUseStaminaTime[playerId] = nil
	end
	return true
end

creatureevent:register()