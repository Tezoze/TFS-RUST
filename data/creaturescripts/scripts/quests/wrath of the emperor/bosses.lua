local bosses = {
	['fury of the emperor'] =  {position = Position(33048, 31085, 15), storage = GlobalStorage.WrathOfTheEmperor.Bosses.Fury},
	['wrath of the emperor'] = {position = Position(33094, 31087, 15), storage = GlobalStorage.WrathOfTheEmperor.Bosses.Wrath},
	['scorn of the emperor'] = {position = Position(33095, 31110, 15), storage = GlobalStorage.WrathOfTheEmperor.Bosses.Scorn},
	['spite of the emperor'] = {position = Position(33048, 31111, 15), storage = GlobalStorage.WrathOfTheEmperor.Bosses.Spite},
}

-- Adding duplicate prevention for boss kills
local recent_boss_kills = {}

function onKill(creature, target)
	local player = creature:getPlayer()
	if not player then
		return true
	end

	if not target then
		return true
	end

	local bossConfig = bosses[target:getName():lower()]
	if not bossConfig then
		return true
	end

	-- Adding duplicate prevention for boss kills
	local targetId = target:getId()
	local playerGuid = player:getGuid()

	if not recent_boss_kills[playerGuid] then
		recent_boss_kills[playerGuid] = {}
	end

	if recent_boss_kills[playerGuid][targetId] then
		return true
	end

	recent_boss_kills[playerGuid][targetId] = true

	addEvent(function()
		if recent_boss_kills[playerGuid] then
			recent_boss_kills[playerGuid][targetId] = nil
		end
	end, 3000)

	Game.setStorageValue(bossConfig.storage, 0)

	-- Give quest progress to all players in the area who are on Mission 10
	local spectators = Game.getSpectators(bossConfig.position, false, true, 10, 10, 10, 10)
	for i, spectator in ipairs(spectators) do
		if spectator:isPlayer() then
			local questline = spectator:getStorageValue(Storage.WrathoftheEmperor.Questline) or 0
			local mission10 = spectator:getStorageValue(Storage.WrathoftheEmperor.Mission10) or 0
			if questline >= 29 and mission10 >= 1 and mission10 <= 5 then
				local newProgress = mission10 + 1
				spectator:setStorageValue(Storage.WrathoftheEmperor.Mission10, newProgress)
				spectator:sendTextMessage(MESSAGE_EVENT_ADVANCE, 'You have destroyed ' .. newProgress .. ' of 4 emperor\'s influences!')

				if newProgress == 6 then
					spectator:sendTextMessage(MESSAGE_EVENT_ADVANCE, 'You have destroyed all emperor\'s influences! Return to Zizzle!')
				end
			end
		end
	end

	local tile = Tile(bossConfig.position)
	if tile then
		local thing = tile:getItemById(11753)
		if thing then
			thing:transform(12383)
		end
	end
	return true
end
