-- Lorek - Converted from XML to Lua NpcType
-- Original XML: data/npc/Lorek.xml
-- Original Script: data/npc/scripts/Lorek.lua

local npcName = "Lorek"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a lorek")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(2000)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 132, lookHead = 19, lookBody = 10, lookLegs = 38, lookFeet = 95})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)


-- Travel
local function addTravelKeyword(keyword, text, cost, destination, condition)
	local travelKeyword = keywordHandler:addKeyword({keyword}, StdModule.say, {npcHandler = npcHandler, text = 'Do you seek a passage to ' .. (text or keyword:titleCase()) .. ' for |TRAVELCOST|?', cost = cost}, condition and function(player) return player:getPawAndFurRank() >= 3 end or nil)
		travelKeyword:addChildKeyword({'yes'}, StdModule.travel, {npcHandler = npcHandler, premium = false, cost = cost, destination = destination})
		travelKeyword:addChildKeyword({'no'}, StdModule.say, {npcHandler = npcHandler, text = 'Maybe another time.', reset = true})
end

addTravelKeyword('west', 'the west end of Port Hope', 7, Position(32558, 32780, 7))
addTravelKeyword('centre', 'the centre of Port Hope', 7, Position(32628, 32771, 7))
addTravelKeyword('darama', nil, 30, Position(32987, 32729, 7))
addTravelKeyword('center', 'the centre of Port Hope', 0, Position(32628, 32771, 7))
addTravelKeyword('chor', nil, 30, Position(32968, 32799, 7), true)
addTravelKeyword('banuta', nil, 30, Position(32826, 32631, 7), true)
addTravelKeyword('mountain', nil, 30, Position(32987, 32729, 7), true)
addTravelKeyword('mountain pass', nil, 30, Position(32987, 32729, 7), true)


-- Basic
keywordHandler:addKeyword({'ferumbras'}, StdModule.say, {npcHandler = npcHandler, text = "I heard he is some scary magician or so."})
keywordHandler:addKeyword({'passage'}, StdModule.say, {npcHandler = npcHandler, text = 'I can travel you to west, centre, darama, chor or banuta.'})


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
