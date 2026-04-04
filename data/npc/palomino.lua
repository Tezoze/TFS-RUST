-- Palomino - Converted from XML to Lua NpcType
-- Original XML: data/npc/Palomino.xml
-- Original Script: data/npc/scripts/Palomino.lua

local npcName = "Palomino"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a palomino")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(2000)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 128, lookHead = 116, lookBody = 39, lookLegs = 12, lookFeet = 97, lookAddons = 2})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)


local function creatureSayCallback(cid, type, msg)
	if not npcHandler:isFocused(cid) then
		return false
	end

	if msgcontains(msg, 'transport') then
		npcHandler:say('We can bring you to Venore with one of our coaches for 125 gold. Are you interested?', cid)
		npcHandler.topic[cid] = 1
	elseif isInArray({'rent', 'horses'}, msg) then
		npcHandler:say('Do you want to rent a horse for one day at a price of 500 gold?', cid)
		npcHandler.topic[cid] = 2
	elseif msgcontains(msg, 'yes') then
		local player = Player(cid)
		if npcHandler.topic[cid] == 1 then
			if player:isPzLocked() then
				npcHandler:say('First get rid of those blood stains!', cid)
				return true
			end

			if not player:removeMoneyNpc(125) then
				npcHandler:say('You don\'t have enough money.', cid)
				return true
			end

			player:getPosition():sendMagicEffect(CONST_ME_TELEPORT)
			local destination = Position(32850, 32124, 7)
			player:teleportTo(destination)
			destination:sendMagicEffect(CONST_ME_TELEPORT)
			npcHandler:say('Have a nice trip!', cid)
		elseif npcHandler.topic[cid] == 2 then
			if player:getStorageValue(Storage.RentedHorseTimer) >= os.time() then
				npcHandler:say('You already have a horse.', cid)
				return true
			end

			if not player:removeMoneyNpc(500) then
				npcHandler:say('You do not have enough money to rent a horse!', cid)
				return true
			end

			local mountId = {22, 25, 26}
			player:addMount(mountId[math.random(#mountId)])
			player:setStorageValue(Storage.RentedHorseTimer, os.time() + 86400)
			player:addAchievement('Natural Born Cowboy')
			npcHandler:say('I\'ll give you one of our experienced ones. Take care! Look out for low hanging branches.', cid)
		end
		npcHandler.topic[cid] = 0
	elseif msgcontains(msg, 'no') and npcHandler.topic[cid] > 0 then
		npcHandler:say('Then not.', cid)
		npcHandler.topic[cid] = 0
	end
	return true
end

npcHandler:setMessage(MESSAGE_GREET, 'Salutations, |PLAYERNAME| I guess you are here for the {horses}.')

npcHandler:setCallback(CALLBACK_MESSAGE_DEFAULT, creatureSayCallback)


-- NpcType callbacks (MUST call setCurrentNpc first!)
npcType:eventType(NPCS_EVENT_APPEAR)
npcType:onAppear(function(npc, creature)
    setCurrentNpc(npc)
    npcHandler:onCreatureAppear(creature)
end)

npcType:eventType(NPCS_EVENT_DISAPPEAR)
npcType:onDisappear(function(npc, creature)
    setCurrentNpc(npc)
    npcHandler:onCreatureDisappear(creature)
end)

npcType:eventType(NPCS_EVENT_SAY)
npcType:onSay(function(npc, creature, type, message)
    setCurrentNpc(npc)
    npcHandler:onCreatureSay(creature, type, message)
end)

npcType:eventType(NPCS_EVENT_THINK)
npcType:onThink(function(npc, interval)
    setCurrentNpc(npc)
    npcHandler:onThink()
end)

npcType:eventType(NPCS_EVENT_CLOSECHANNEL)
npcType:onCloseChannel(function(npc, creature)
    setCurrentNpc(npc)
    npcHandler:onPlayerCloseChannel(creature)
end)

npcHandler:addModule(FocusModule:new())
npcType:register()
