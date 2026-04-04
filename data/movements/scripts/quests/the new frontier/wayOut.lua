function onStepIn(creature, item, position, fromPosition)
	local player = creature:getPlayer()
	if not player then
		return true
	end

	-- Only trigger if player is on Mission 01 step 1 (just started via Ongulf)
	-- This prevents triggering for players who used access tokens (Questline >= 25)
	local questline = player:getStorageValue(Storage.TheNewFrontier.Questline)
	local mission01 = player:getStorageValue(Storage.TheNewFrontier.Mission01)
	
	if questline == 1 and mission01 == 1 then
		--Questlog, The New Frontier Quest "Mission 01: New Land"
		player:setStorageValue(Storage.TheNewFrontier.Mission01, 2)
		player:setStorageValue(Storage.TheNewFrontier.Questline, 2)
		player:say("You have found the passage through the mountains and can report about your success.", TALKTYPE_MONSTER_SAY)
	end
	return true
end
