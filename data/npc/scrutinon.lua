-- Scrutinon - Converted from XML to Lua NpcType
-- Original XML: data/npc/Scrutinon.xml
-- Original Script: data/npc/scripts/Scrutinon.lua

local npcName = "Scrutinon"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a scrutinon")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(0)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookTypeEx = 7825})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)



-- Travel
local function addTravelKeyword(keyword, destination)
	local travelKeyword = keywordHandler:addKeyword({keyword}, StdModule.say, {npcHandler = npcHandler, text = 'Do you want to sail ' .. keyword:titleCase() .. '?'})
		travelKeyword:addChildKeyword({'yes'}, StdModule.travel, {npcHandler = npcHandler, premium = false, cost = 0, destination = destination})
		travelKeyword:addChildKeyword({'no'}, StdModule.say, {npcHandler = npcHandler, text = 'We would like to serve you some time.', reset = true})
end

addTravelKeyword('ab\'dendriel', Position(32734, 31668, 6))
addTravelKeyword('edron', Position(33175, 31764, 6))
addTravelKeyword('venore', Position(32954, 32022, 6))
addTravelKeyword('darashia', Position(33289, 32480, 6))

-- Basic
keywordHandler:addKeyword({'name'}, StdModule.say, {npcHandler = npcHandler,
	text = {
		"My name is Scrutinon. However, there are not many people calling my name nowadays. Not many captains even dare to land on this island. It is too close to {Quirefang}. ...",
		"Most of them do not know this island by that name. Some call it Demon Horn, others the Dragon's Tooth or the Gray Beach as none of them ever came closer than a fair distance. ...",
		"There are drifts and storms surrounding that place that are far too dangerous to navigate through even for the most versed captains. They often sail not closer than to this island here and drop off whoever dares to explore near this dreaded coast."
	}}
)
keywordHandler:addKeyword({'quirefang'}, StdModule.say, {npcHandler = npcHandler,
	text = {
		"This island is cleft. Go there only prepared or you will meet your end. The surface of this forgotten rock is a barren wasteland full of hostile creatures. ...",
		"Its visage is covered with holes and tunnels in which its leggy inhabitants are hiding. Its bowels filled with the strangest creatures, waiting to feast on whatever dares to disturb their hive. ...",
		"And you will find no shelter in Quirefang's black depths, where the creatures of the deep are fulfilling a dark prophecy. ...",
		"It is impossible to reach it by ship or boat. However, there was one before you. A {visitor} who found a way to enter the island."
	}}
)


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
