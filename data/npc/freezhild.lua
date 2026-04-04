-- Freezhild - Converted from XML to Lua NpcType
-- Original XML: data/npc/Freezhild.xml
-- Original Script: data/npc/scripts/Freezhild.lua

local npcName = "Freezhild"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a freezhild")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(2000)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 149, lookBody = 47, lookLegs = 105, lookFeet = 105})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)


local function creatureSayCallback(cid, type, msg)
	if not npcHandler:isFocused(cid) then
		return false
	end

	local player = Player(cid)

	if msgcontains(msg, "weapons") then
		if player:getStorageValue(Storage.secretService.AVINMission06) == 1 then
			npcHandler:say("Crate of weapons you say.. for me?", cid)
			npcHandler.topic[cid] = 1
		end
	elseif msgcontains(msg, "yes") then
		if npcHandler.topic[cid] == 1 then
			if player:removeItem(7707, 1) then
				player:setStorageValue(Storage.secretService.AVINMission06, 2)
				player:setStorageValue(12567, 2) -- Mission 6: Weapons delivered to Freezhild
				npcHandler:say("Why thank you " .. player:getName() .. ".", cid)
			else
				npcHandler:say("You don't have any crate of weapons!", cid)
			end
			npcHandler.topic[cid] = 0
		end
	end
	return true
end

npcHandler:setMessage(MESSAGE_WALKAWAY, "I hope you have a cold day, friend.")
npcHandler:setMessage(MESSAGE_FAREWELL, "I hope you have a cold day, friend.")
npcHandler:setMessage(MESSAGE_GREET, "Welcome, to my cool home.")
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
