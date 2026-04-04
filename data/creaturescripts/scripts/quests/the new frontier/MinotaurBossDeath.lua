function onDeath(creature, corpse, killer, mostDamageKiller, lastHitUnjustified, mostDamageUnjustified)
	local player = creature:getPlayer()
	if not player then
		return true
	end

	-- If player dies during the minotaur boss test (questline 19), reset to allow retry
	if player:getStorageValue(Storage.TheNewFrontier.Questline) == 19 then
		player:setStorageValue(Storage.TheNewFrontier.Questline, 18)
		player:sendTextMessage(MESSAGE_STATUS_WARNING, 'You failed the minotaur boss test. You can try again.')
	end
	return true
end
