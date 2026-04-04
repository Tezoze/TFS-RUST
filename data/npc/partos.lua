-- Partos - Converted from XML to Lua NpcType
-- Original XML: data/npc/Partos.xml
-- Original Script: data/npc/scripts/Partos.lua

local npcName = "Partos"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a partos")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(2000)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 128, lookHead = 116, lookBody = 56, lookLegs = 95, lookFeet = 121})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)


local function creatureSayCallback(cid, type, msg)
	if not npcHandler:isFocused(cid) then
		return false
	end

	local player = Player(cid)
	if msgcontains(msg, 'supplies') then
		if player:getStorageValue(Storage.DjinnWar.EfreetFaction.Mission01) == 1 then
			npcHandler:say({
				'What!? I bet, Baa\'leal sent you! ...',
				'I won\'t tell you anything! Shove off!'
			}, cid)
			player:setStorageValue(Storage.DjinnWar.EfreetFaction.Mission01, 2)
		else
			npcHandler:say('I won\'t talk about that.', cid)
		end

	elseif msgcontains(msg, 'ankrahmun') then
		npcHandler:say({
			'Yes, I\'ve lived in Ankrahmun for quite some time. Ahh, good old times! ...',
			'Unfortunately I had to relocate. <sigh> ...',
			'Business reasons - you know.'
		}, cid)
	end
	return true
end

keywordHandler:addKeyword({'prison'}, StdModule.say, {npcHandler = npcHandler, text = 'You mean that\'s a JAIL? They told me it\'s the finest hotel in town! THAT explains the lousy roomservice!'})
keywordHandler:addKeyword({'jail'}, StdModule.say, {npcHandler = npcHandler, text = 'You mean that\'s a JAIL? They told me it\'s the finest hotel in town! THAT explains the lousy roomservice!'})
keywordHandler:addKeyword({'cell'}, StdModule.say, {npcHandler = npcHandler, text = 'You mean that\'s a JAIL? They told me it\'s the finest hotel in town! THAT explains the lousy roomservice!'})

npcHandler:setMessage(MESSAGE_GREET, 'Welcome to my little kingdom, |PLAYERNAME|.')
npcHandler:setMessage(MESSAGE_FAREWELL, 'Good bye, visit me again. I will be here, promised.')
npcHandler:setMessage(MESSAGE_WALKAWAY, 'Good bye, visit me again. I will be here, promised.')

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
