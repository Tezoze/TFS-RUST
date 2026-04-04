local waves = {
	Position(32779, 31166, 10),
	Position(32787, 31166, 10),
	Position(32782, 31162, 10),
	Position(32784, 31162, 10),
	Position(32782, 31170, 10),
	Position(32784, 31170, 10)
}

local creatureNames = {
	[1] = 'rift worm',
	[2] = 'rift brood',
	[3] = 'rift scythe',
	[4] = 'war golem'
}

local effectPositions = {
	Position(32779, 31161, 10),
	Position(32787, 31171, 10)
}

local function doClearAreaAzerus()
	if Game.getStorageValue(GlobalStorage.InServiceOfYalahar.LastFight) == 1 then
		local spectators, spectator = Game.getSpectators(Position(32783, 31166, 10), false, false, 10, 10, 10, 10)
		for i = 1, #spectators do
			spectator = spectators[i]
			if spectator:isMonster() then
				spectator:getPosition():sendMagicEffect(CONST_ME_POFF)
				spectator:remove()
			elseif spectator:isPlayer() then
				-- Unregister Azerus kill event
				spectator:unregisterEvent("ServiceOfYalaharAzerus")
			end
		end
		Game.setStorageValue(GlobalStorage.InServiceOfYalahar.LastFight, 0)
	end
	return true
end

local function doChangeAzerus()
	local spectators, spectator = Game.getSpectators(Position(32783, 31166, 10), false, false, 10, 10, 10, 10)
	for i = 1, #spectators do
		spectator = spectators[i]
		if spectator:isMonster() and spectator:getName():lower() == "azerus" then
			spectator:say("No! I am losing my energy!", TALKTYPE_MONSTER_SAY)
			local newMonster = Game.createMonster("Azerus", spectator:getPosition(), false, true)
			if newMonster then
				newMonster:registerEvent("ServiceOfYalaharAzerus")
			end
			spectator:remove()
			return true
		end
	end
	return false
end

local function summonMonster(name, position)
Game.createMonster(name, position, false, true)
	--Game.createMonster(name, position)
	position:sendMagicEffect(CONST_ME_TELEPORT)
end

function onUse(player, item, fromPosition, target, toPosition, isHotkey)
	if Game.getStorageValue(GlobalStorage.InServiceOfYalahar.LastFight) == 1 then
		player:say('You have to wait some time before this globe charges.', TALKTYPE_MONSTER_SAY)
		return true
	end

	-- Check if there are any monsters still in the area from a previous fight
	local spectators = Game.getSpectators(Position(32783, 31166, 10), false, false, 10, 10, 10, 10)
	for i = 1, #spectators do
		if spectators[i]:isMonster() then
			player:say('The previous fight is still ongoing. Please wait for it to end.', TALKTYPE_MONSTER_SAY)
			return true
		end
	end

	local amountOfPlayers = 1
	local spectators = Game.getSpectators(Position(32783, 31166, 10), false, true, 10, 10, 10, 10)
	if #spectators < amountOfPlayers then
		for i = 1, #spectators do
			spectators[i]:sendTextMessage(MESSAGE_EVENT_ADVANCE, "You need atleast " .. amountOfPlayers .. " players inside the quest room.")
		end
		return true
	end

	Game.setStorageValue(GlobalStorage.InServiceOfYalahar.LastFight, 1)

	-- Register Azerus kill event for all players in the area
	local spectators = Game.getSpectators(Position(32783, 31166, 10), false, true, 10, 10, 10, 10)
	for i = 1, #spectators do
		spectators[i]:registerEvent("ServiceOfYalaharAzerus")
	end

	addEvent(function()
		local monster = Game.createMonster("Azerus2", Position(32783, 31167, 10), false, true)
		if monster then
			monster:registerEvent("ServiceOfYalaharAzerus")
		end
	end, 18 * 1000)
	--addEvent(Game.createMonster, 18 * 1000, "Azerus2", Position(32783, 31167, 10))

	local azeruswavemonster
	for i = 1, #creatureNames do
		azeruswavemonster = creatureNames[i]
		for k = 1, #waves do
			addEvent(summonMonster, (i - 1) * 60 * 1000, azeruswavemonster, waves[k])
		end
	end

	for i = 1, #effectPositions do
		effectPositions[i]:sendMagicEffect(CONST_ME_HOLYAREA)
	end

	addEvent(doChangeAzerus, 3 * 60 * 1000) -- Change Azerus when war golems spawn
	addEvent(doClearAreaAzerus, 5 * 60 * 1000)
	return true
end
