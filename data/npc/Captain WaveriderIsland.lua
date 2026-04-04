-- Captain WaveriderIsland - Converted from XML to Lua NpcType
-- Original XML: data/npc/Captain WaveriderIsland.xml
-- Original Script: data/npc/scripts/Captain WaveriderIsland.lua

local npcId = "Captain WaveriderIsland"  -- Unique ID for spawn system
local npcDisplayName = "Captain Waverider"  -- Name shown in-game
local npcType = Game.createNpcType(npcId)

-- NPC Properties (from XML)
npcType:name(npcDisplayName)
npcType:nameDescription("a captain waverider")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(2000)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 96})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Per-NPC handler storage to prevent state sharing between multiple instances
local npcHandlers = {}

local function getHandlers(npc)
	local npcId = npc:getId()
	if not npcHandlers[npcId] then
		npcHandlers[npcId] = {
			keywordHandler = KeywordHandler:new(),
			npcHandler = nil
		}
		npcHandlers[npcId].npcHandler = NpcHandler:new(npcHandlers[npcId].keywordHandler)
		
		local handler = npcHandlers[npcId].npcHandler
		local kh = npcHandlers[npcId].keywordHandler
		
		local travelNode = kh:addKeyword({'liberty bay'}, StdModule.say, {npcHandler = handler, text = 'Do you seek a passage back to Liberty Bay for |TRAVELCOST|?', cost = 0, discount = 'postman'})
			travelNode:addChildKeyword({'yes'}, StdModule.travel, {npcHandler = handler, premium = false, cost = 50, discount = 'postman', destination = Position(32349, 32856, 7) })
			travelNode:addChildKeyword({'no'}, StdModule.say, {npcHandler = handler, reset = true, text = 'We would like to serve you some time.'})
		
		kh:addKeyword({'passage'}, StdModule.say, {npcHandler = handler, text = 'Where do you want to go? To {Liberty bay}?'})
		kh:addKeyword({'job'}, StdModule.say, {npcHandler = handler, text = 'I am the captain of this ship.'})
		kh:addKeyword({'captain'}, StdModule.say, {npcHandler = handler, text = 'I am the captain of this ship.'})
		
		handler:setMessage(MESSAGE_GREET, "Greetings, daring adventurer. If you need a return {passage}, let me know.")
		handler:setMessage(MESSAGE_FAREWELL, "Good bye.")
		handler:setMessage(MESSAGE_WALKAWAY, "Oh well.")
		
		handler:addModule(FocusModule:new())
	end
	return npcHandlers[npcId]
end

-- NpcType callbacks (MUST call setCurrentNpc first!)
npcType:eventType(NPCS_EVENT_APPEAR)
npcType:onAppear(function(npc, creature)
    setCurrentNpc(npc)
    local handlers = getHandlers(npc)
    handlers.npcHandler:onCreatureAppear(creature)
end)

npcType:eventType(NPCS_EVENT_DISAPPEAR)
npcType:onDisappear(function(npc, creature)
    setCurrentNpc(npc)
    local handlers = getHandlers(npc)
    handlers.npcHandler:onCreatureDisappear(creature)
end)

npcType:eventType(NPCS_EVENT_SAY)
npcType:onSay(function(npc, creature, type, message)
    setCurrentNpc(npc)
    local handlers = getHandlers(npc)
    handlers.npcHandler:onCreatureSay(creature, type, message)
end)

npcType:eventType(NPCS_EVENT_THINK)
npcType:onThink(function(npc, interval)
    setCurrentNpc(npc)
    local handlers = getHandlers(npc)
    handlers.npcHandler:onThink()
end)

npcType:eventType(NPCS_EVENT_CLOSECHANNEL)
npcType:onCloseChannel(function(npc, creature)
    setCurrentNpc(npc)
    local handlers = getHandlers(npc)
    handlers.npcHandler:onPlayerCloseChannel(creature)
end)

npcType:register()
