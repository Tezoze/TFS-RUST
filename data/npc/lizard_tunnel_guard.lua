-- Lizard Tunnel Guard - Converted from XML to Lua NpcType
-- Original XML: data/npc/Lizard Tunnel Guard.xml
-- Original Script: data/npc/scripts/Lizard Tunnel Guard.lua

local npcName = "Lizard Tunnel Guard"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a lizard tunnel guard")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(0)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 338})
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
		npcHandlers[npcId].npcHandler:setCallback(CALLBACK_GREET, greetCallback)
		npcHandlers[npcId].npcHandler:addModule(FocusModule:new())
	end
	return npcHandlers[npcId]
end


local function greetCallback(cid)
	local player = Player(cid)
	if player:getStorageValue(Storage.WrathoftheEmperor.Questline) >= 2 then
		player:setStorageValue(Storage.WrathoftheEmperor.GuardcaughtYou, 1)
		player:setStorageValue(Storage.WrathoftheEmperor.CrateStatus, 0)
		player:teleportTo(Position(33361, 31206, 8))
		player:say("The guards have spotted you. You were forcibly dragged into a small cell. It looks like you need to build another disguise.", TALKTYPE_MONSTER_SAY)
	end
	return true
end

-- Callback now set in getHandlers()


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

-- FocusModule now added in getHandlers()
npcType:register()
