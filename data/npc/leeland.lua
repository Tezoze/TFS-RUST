-- Leeland - Converted from XML to Lua NpcType
-- Original XML: data/npc/Leeland.xml
-- Original Script: data/npc/scripts/Leeland.lua

local npcName = "Leeland"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a leeland")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(2000)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 130, lookHead = 19, lookBody = 53, lookLegs = 15, lookFeet = 95})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)


local function creatureSayCallback(cid, type, msg)
	local player = Player(cid)
	if not npcHandler:isFocused(cid) then
		return false
	--The New Frontier
	elseif msgcontains(msg, "farmine") then
		if player:getStorageValue(Storage.TheNewFrontier.Questline) == 15 then
			npcHandler:say("Oh yes, that project the whole dwarven community is so excited about. I guess I already know why you are here, but speak up.", cid)
			npcHandler.topic[cid] = 1
		end
	elseif msgcontains(msg, "impress") or msgcontains(msg, "plea") then
		if npcHandler.topic[cid] == 1 then
			if player:getStorageValue(Storage.TheNewFrontier.BribeLeeland) < 1 then
				npcHandler:say("The idea of a promising market and new resources suits us quite well. I think it is reasonable to send some assistance.", cid)
				player:setStorageValue(Storage.TheNewFrontier.BribeLeeland, 1)
				player:setStorageValue(Storage.TheNewFrontier.Mission05, player:getStorageValue(Storage.TheNewFrontier.Mission05) + 1) --Questlog, The New Frontier Quest "Mission 05: Getting Things Busy"
			end
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
