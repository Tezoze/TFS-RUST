-- Maritima - Converted from XML to Lua NpcType
-- Original XML: data/npc/Maritima.xml
-- Original Script: data/npc/scripts/Maritima.lua

local npcName = "Maritima"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a maritima")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(0)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookTypeEx = 5811})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)


local function creatureSayCallback(cid, type, msg)
	if not npcHandler:isFocused(cid) then
		return false
	end

	local player = Player(cid)
	if(msgcontains(msg, "quara")) then
		if((player:getStorageValue(Storage.InServiceofYalahar.Questline) or 0) == 41 and (player:getStorageValue(Storage.InServiceofYalahar.QuaraInky) or 0) < 1  and (player:getStorageValue(Storage.InServiceofYalahar.QuaraSplasher) or 0) < 1 and (player:getStorageValue(Storage.InServiceofYalahar.QuaraSharptooth) or 0) < 1) then
			npcHandler:say({
				"The quara in this area are a strange race that seeks for inner perfection rather than physical one. ...",
				"However, recently the quara got mad because their area is flooded with toxic sewage from the city. If you could inform someone about it, they might stop the sewage and the quara could return to their own business."
			}, cid)
			player:setStorageValue(Storage.InServiceofYalahar.Questline, 42)
			player:setStorageValue(Storage.InServiceofYalahar.Mission07, 3) -- StorageValue for Questlog "Mission 07: A Fishy Mission"
			player:setStorageValue(Storage.InServiceofYalahar.QuaraState, 1)
			npcHandler.topic[cid] = 0
		end
	end
	return true
end

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
