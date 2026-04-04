-- Maris Fenrock - Converted from XML to Lua NpcType
-- Original XML: data/npc/Maris Fenrock.xml
-- Original Script: data/npc/scripts/MarisFenrock.lua

local npcId = "Maris Fenrock"  -- Unique ID for spawn system
local npcDisplayName = "Maris"  -- Name shown in-game
local npcType = Game.createNpcType(npcId)

-- NPC Properties (from XML)
npcType:name(npcDisplayName)
npcType:nameDescription("a maris")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(2000)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 151, lookHead = 78, lookBody = 51, lookLegs = 47})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)


-- Travel
local function addTravelKeyword(keyword, text, cost, destination)
	local travelKeyword = keywordHandler:addKeyword({keyword}, StdModule.say, {npcHandler = npcHandler, text = 'Do you want go to ' .. text .. ' for |TRAVELCOST|?', cost = cost})
		travelKeyword:addChildKeyword({'yes'}, StdModule.travel, {npcHandler = npcHandler, premium = false, cost = cost, destination = destination})
		travelKeyword:addChildKeyword({'no'}, StdModule.say, {npcHandler = npcHandler, text = 'Alright then!', reset = true})
end

addTravelKeyword('yalahar', 'back to Yalahar', 100, Position(32649, 31292, 6))
addTravelKeyword('mistrock', 'to the Mistrock', 50, Position(32640, 31439, 7))
addTravelKeyword('fenrock', 'to the Fenrock', 100, Position(32563, 31313, 7))

-- Kick
--keywordHandler:addKeyword({'kick'}, StdModule.kick, {npcHandler = npcHandler, destination = Position(32561, 31312, 7)})

-- Basic
keywordHandler:addKeyword({'offer'}, StdModule.say, {npcHandler = npcHandler, text = 'I can take you to {Yalahar} or {Mistrock}!'})
keywordHandler:addKeyword({'passage'}, StdModule.say, {npcHandler = npcHandler, text = 'I can take you to {Yalahar} or {Mistrock}!'})
keywordHandler:addKeyword({'job'}, StdModule.say, {npcHandler = npcHandler, text = 'I am Maris, Captain of this ship.'})
keywordHandler:addKeyword({'captain'}, StdModule.say, {npcHandler = npcHandler, text = 'I am Maris, Captain of this ship.'})

npcHandler:setMessage(MESSAGE_GREET, "Oh, you're still alive. Hello, |PLAYERNAME|.")
npcHandler:setMessage(MESSAGE_FAREWELL, "Yeah, bye or whatever.")
npcHandler:setMessage(MESSAGE_WALKAWAY, "Bye.")


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
