-- Theodore Loveless - Converted from XML to Lua NpcType
-- Original XML: data/npc/Theodore Loveless.xml
-- Original Script: data/npc/scripts/Theodore Loveless.lua

local npcName = "Theodore Loveless"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a theodore loveless")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(2000)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 132, lookHead = 19, lookBody = 95, lookLegs = 76, lookFeet = 114, lookAddons = 1})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)


local function creatureSayCallback(cid, type, msg)
	if not npcHandler:isFocused(cid) then
		return false
	end

	if msgcontains(msg, 'cigar') then
		npcHandler:say('Oh my. Have you gotten an exquisite cigar for me, my young friend?', cid)
		npcHandler.topic[cid] = 1
	elseif msgcontains(msg, 'yes') and npcHandler.topic[cid] == 1 then
		local player = Player(cid)
		if not player:removeItem(7499, 1) then
			npcHandler.topic[cid] = 0
			return true
		end

		player:setStorageValue(Storage.WhatAFoolishQuest.Cigar, 1)
		Npc():getPosition():sendMagicEffect(CONST_ME_EXPLOSIONHIT)
		npcHandler:say({
			'Ah what a fine blend. I really ...',
			'OUCH! What have you done you fool? How dare you???'
		}, cid)
		npcHandler.topic[cid] = 0
	elseif msgcontains(msg, 'no') and npcHandler.topic[cid] == 1 then
		npcHandler:say('Oh, then there must be a misunderstanding.', cid)
		npcHandler.topic[cid] = 0
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
