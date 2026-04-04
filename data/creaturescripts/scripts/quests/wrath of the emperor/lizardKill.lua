local lizardKill

-- Adding the kill tracking table
local recent_kills = {}

function onKill(creature, target)
	local player = creature:getPlayer()
	if not player then
		return true
	end

	-- Define storage mapping here when Storage is available
	if not lizardKill then
		lizardKill = {
			['lizard magistratus'] = Storage.WrathoftheEmperor.Mission06,
			['lizard noble'] = Storage.WrathoftheEmperor.Mission07
		}
	end

	-- Check if target is a player or has a master (summoned creature)
	if target:isPlayer() or target:getMaster() then
		return true
	end

-- Adding duplicate prevention
	local targetId = target:getId()
	local playerGuid = player:getGuid()
	
	if not recent_kills[playerGuid] then
		recent_kills[playerGuid] = {}
	end
	
	if recent_kills[playerGuid][targetId] then
		return true
	end
	
	recent_kills[playerGuid][targetId] = true
	
	addEvent(function()
		if recent_kills[playerGuid] then
			recent_kills[playerGuid][targetId] = nil
		end
	end, 3000)

	local targetName = target:getName():lower()
	local bossStorage = lizardKill[targetName]
	if not bossStorage then
		return true
	end

	-- Check if player is on the correct quest stage
	local questline = player:getStorageValue(Storage.WrathoftheEmperor.Questline) or 0
	if questline < 22 then
		return true
	end

	-- Handle lizard magistratus kills (Mission06: kill 4)
	if targetName == 'lizard magistratus' then
		local currentKills = player:getStorageValue(bossStorage) or 0
		if currentKills < 4 then
			player:setStorageValue(bossStorage, currentKills + 1)
			player:sendTextMessage(MESSAGE_EVENT_ADVANCE, 'You have slain ' .. target:getName() .. '!')
			if currentKills + 1 == 4 then
				player:sendTextMessage(MESSAGE_EVENT_ADVANCE, 'You have slain enough Lizard Magistratus. Return to Zlak!')
			end
		end
	-- Handle lizard noble kills (Mission07: kill 6)
	elseif targetName == 'lizard noble' then
		local currentKills = player:getStorageValue(bossStorage) or 0
		if currentKills < 6 then
			player:setStorageValue(bossStorage, currentKills + 1)
			player:sendTextMessage(MESSAGE_EVENT_ADVANCE, 'You have slain ' .. target:getName() .. '!')
			if currentKills + 1 == 6 then
				player:sendTextMessage(MESSAGE_EVENT_ADVANCE, 'You have slain enough Lizard Nobles. Return to Zlak!')
			end
		end
	end

	return true
end
