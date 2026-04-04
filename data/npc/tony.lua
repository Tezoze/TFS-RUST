-- Tony - Converted from XML to Lua NpcType
-- Original XML: data/npc/Tony.xml
-- Original Script: data/npc/scripts/Tony.lua

local npcName = "Tony"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a tony")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(2000)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 131, lookHead = 58, lookBody = 43, lookLegs = 38, lookFeet = 76})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)


local function creatureSayCallback(cid, type, msg)
	if not npcHandler:isFocused(cid) then
		return false
	end

	local player = Player(cid)
	if msgcontains(msg, "report") then
		if player:getStorageValue(Storage.InServiceofYalahar.Questline) == 7 or player:getStorageValue(Storage.InServiceofYalahar.Questline) == 13 then
			npcHandler:say("Uhm, report, eh? <slowly gives a clumsy description of recent problems>. ", cid)
			player:setStorageValue(Storage.InServiceofYalahar.Questline, math.max(1, player:getStorageValue(Storage.InServiceofYalahar.Questline) +1))
			player:setStorageValue(Storage.InServiceofYalahar.Mission02, math.max(1, player:getStorageValue(Storage.InServiceofYalahar.Mission02) +1)) -- StorageValue for Questlog "Mission 02: Watching the Watchmen"
			npcHandler.topic[cid] = 0
		end
	elseif msgcontains(msg, "pass") then
		npcHandler:say("You can {pass} either to the {Arena Quarter} or {Foreigner Quarter}. Which one will it be?", cid)
		npcHandler.topic[cid] = 1
	elseif(msgcontains(msg, "arena")) then
		if npcHandler.topic[cid] == 1 then
			player:teleportTo(Position(32695, 31254, 7))
			player:getPosition():sendMagicEffect(CONST_ME_TELEPORT)
			npcHandler.topic[cid] = 0
		end
	elseif(msgcontains(msg, "foreigner")) then
		if npcHandler.topic[cid] == 1 then
			player:teleportTo(Position(32695, 31259, 7))
			player:getPosition():sendMagicEffect(CONST_ME_TELEPORT)
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
