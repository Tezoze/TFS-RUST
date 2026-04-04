-- Harlow Vengoth - Converted from XML to Lua NpcType
-- Original XML: data/npc/Harlow Vengoth.xml
-- Original Script: data/npc/scripts/Harlow Vengoth.lua

local npcId = "Harlow Vengoth"  -- Unique ID for spawn system
local npcDisplayName = "Harlow"  -- Name shown in-game
local npcType = Game.createNpcType(npcId)

-- NPC Properties (from XML)
npcType:name(npcDisplayName)
npcType:nameDescription("a harlow")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(0)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 151, lookHead = 116, lookBody = 77, lookLegs = 94, lookFeet = 97})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)


local travelNode = keywordHandler:addKeyword({'yalahar'}, StdModule.say, {npcHandler = npcHandler, text = 'Do you seek a passage to Yalahar for |TRAVELCOST|?', cost = 50})
	travelNode:addChildKeyword({'yes'}, StdModule.travel, {npcHandler = npcHandler, premium = false, cost = 50, destination = Position(32837, 31366, 7) })
	travelNode:addChildKeyword({'no'}, StdModule.say, {npcHandler = npcHandler, reset = true, text = 'Oh well.'})

npcHandler:setMessage(MESSAGE_GREET, "Want to go back to {Yalahar} for 50 gold? Just ask me.")
npcHandler:setMessage(MESSAGE_FAREWELL, "Good bye.")
npcHandler:setMessage(MESSAGE_WALKAWAY, "Good bye then.")


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
