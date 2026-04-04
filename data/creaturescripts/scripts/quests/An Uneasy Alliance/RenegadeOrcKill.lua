function onKill(creature, target)
	local player = creature:getPlayer()
	if not player then
		return true
	end

	-- Check if target is a player or has a master (summoned creature)
	if target:isPlayer() or target:getMaster() then
		return true
	end

	-- Check if player is on the Uneasy Alliance mission 1
	local uneasyAlliance = player:getStorageValue(Storage.AnUneasyAlliance) or 0
	if uneasyAlliance ~= 1 then
		return true
	end

	local targetName = target:getName():lower()
	if targetName == "renegade orc" then
		player:setStorageValue(Storage.AnUneasyAlliance, 2)
		player:sendTextMessage(MESSAGE_EVENT_ADVANCE, 'You have slain the renegade orc! Report to Curos.')
		player:unregisterEvent("UneasyAllianceRenegadeOrc") -- Deregister renegade orc events after mission completion
	end

	return true
end

