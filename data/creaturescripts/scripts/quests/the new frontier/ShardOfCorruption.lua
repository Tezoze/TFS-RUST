local shardOfCorruption

function onKill(creature, target)
	local player = creature:getPlayer()
	if not player then
		return true
	end

	-- Initialize storage mapping when Storage is available
	if not shardOfCorruption then
		shardOfCorruption = {
			['shard of corruption'] = Storage.TheNewFrontier.Mission04
		}
	end

	-- Check if target is a player or has a master (summoned creature)
	if target:isPlayer() or target:getMaster() then
		return true
	end

	--An Uneasy Alliance
	if target:getName():lower() == 'renegade orc' then
		if player:getStorageValue(Storage.AnUneasyAlliance) == 1 then
			player:setStorageValue(Storage.AnUneasyAlliance, 2)
		end
		return true
	end

	-- Check if player is on the correct quest stage
	local questline = player:getStorageValue(Storage.TheNewFrontier.Questline) or 0
	if questline ~= 12 then
		return true
	end

	local targetName = target:getName():lower()
	local bossStorage = shardOfCorruption[targetName]
	if not bossStorage then
		return true
	end

	-- Only set storage if not already killed
	if (player:getStorageValue(bossStorage) or 0) < 2 then
		player:setStorageValue(bossStorage, 2)
		player:setStorageValue(Storage.TheNewFrontier.Questline, 13)
		player:sendTextMessage(MESSAGE_EVENT_ADVANCE, 'You have slain the Shard of Corruption!')
	end

	return true
end
