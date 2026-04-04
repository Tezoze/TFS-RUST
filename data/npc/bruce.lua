-- Bruce - Converted from XML to Lua NpcType
-- Original XML: data/npc/Bruce.xml
-- Original Script: data/npc/scripts/Bruce.lua

local npcName = "Bruce"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a bruce")
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

	if msgcontains(msg, 'report') then
		local player = Player(cid)
		if isInArray({9, 11}, player:getStorageValue(Storage.InServiceofYalahar.Questline)) then
			npcHandler:say('Well, .. <gives a short and precise report>.', cid)
			player:setStorageValue(Storage.InServiceofYalahar.Questline, player:getStorageValue(Storage.InServiceofYalahar.Questline) + 1)
			player:setStorageValue(Storage.InServiceofYalahar.Mission02, player:getStorageValue(Storage.InServiceofYalahar.Mission02) + 1) -- StorageValue for Questlog 'Mission 02: Watching the Watchmen'
		end
	elseif msgcontains(msg, 'pass') then
		npcHandler:say('You can {pass} either to the {Alchemist Quarter} or {Cemetery Quarter}. Which one will it be?', cid)
		npcHandler.topic[cid] = 1
	elseif npcHandler.topic[cid] == 1 then
		if msgcontains(msg, 'alchemist') then
			local destination = Position(32738, 31113, 7)
			Player(cid):teleportTo(destination)
			destination:sendMagicEffect(CONST_ME_TELEPORT)
			npcHandler.topic[cid] = 0
		elseif msgcontains(msg, 'cemetery') then
			local destination = Position(32743, 31113, 7)
			Player(cid):teleportTo(destination)
			destination:sendMagicEffect(CONST_ME_TELEPORT)
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
