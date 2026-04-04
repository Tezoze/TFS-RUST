function onKill(creature, target)
	local player = creature:getPlayer()
	if not player then
		return true
	end

	if not target:isMonster() then
		return true
	end

	local targetMonster = target

	local pit = player:getStorageValue(Storage.SvargrondArena.Pit)
	if pit < 1 or pit > 10 then
		return true
	end

	local arena = player:getStorageValue(Storage.SvargrondArena.Arena)
	if arena < 1 then
		return true
	end

	-- Check if the killed monster is the SPECIFIC one for this pit (not just any arena creature)
	local expectedMonster = ARENA[arena].creatures[pit]
	if not expectedMonster then
		return true
	end

	if targetMonster:getName():lower() ~= expectedMonster:lower() then
		return true
	end

	-- Remove pillar and create teleport
	local pillarTile = Tile(PITS[pit].pillar)
	if pillarTile then
		local pillarItem = pillarTile:getItemById(SvargrondArena.itemPillar)
		if pillarItem then
			pillarItem:remove()

			local teleportItem = Game.createItem(SvargrondArena.itemTeleport, 1, PITS[pit].tp)
			if teleportItem then
				teleportItem:setActionId(25200)
			end

			SvargrondArena.sendPillarEffect(pit)
		end
	end

	player:setStorageValue(Storage.SvargrondArena.Pit, pit + 1)

	if pit + 1 > 10 then
		-- Player completed the arena, unregister the kill event
		player:unregisterEvent("SvargrondArenaKill")
		player:say('Congratulations! You have completed the arena!', TALKTYPE_MONSTER_SAY)
		-- Set arena completion storage
		local arena = player:getStorageValue(Storage.SvargrondArena.Arena)
		if arena > 0 and ARENA[arena] then
			-- Special storage for arena completions
			if arena == 1 then
				player:setStorageValue(Storage.SvargrondArena.RewardGreenhorn, 1) -- Enable Greenhorn reward chests
				player:setStorageValue(Storage.SvargrondArena.RewardChosenGreenhorn, 1) -- Greenhorn completion
				player:setStorageValue(Storage.SvargrondArena.QuestLogGreenhorn, 2) -- Update quest log for Greenhorn
				player:setStorageValue(26101, 1) -- Door access after Greenhorn completion
			elseif arena == 2 then
				player:setStorageValue(Storage.SvargrondArena.RewardScrapper, 1) -- Enable Scrapper reward chests
				player:setStorageValue(Storage.SvargrondArena.RewardChosenScrapper, 1) -- Scrapper completion
				player:setStorageValue(Storage.SvargrondArena.QuestLogScrapper, 2) -- Update quest log for Scrapper
				player:setStorageValue(27101, 1) -- Door access after Scrapper completion
			elseif arena == 3 then
				player:setStorageValue(Storage.SvargrondArena.RewardWarlord, 1) -- Enable Warlord reward chests
				player:setStorageValue(Storage.SvargrondArena.RewardChosenWarlord, 1) -- Warlord (last arena) completion
				player:setStorageValue(Storage.SvargrondArena.QuestLogWarlord, 2) -- Update quest log for Warlord
				player:setStorageValue(28101, 1) -- Door access after Warlord completion
			end
		end
	else
		player:say('Victory! Head through the new teleporter into the next room.', TALKTYPE_MONSTER_SAY)
	end

	return true
end
