-- CobraNPC - Converted from XML to Lua NpcType
-- Original XML: data/npc/CobraNPC.xml
-- Original Script: data/npc/scripts/Cobra.lua

local npcId = "CobraNPC"  -- Unique ID for spawn system
local npcDisplayName = "Cobra"  -- Name shown in-game
local npcType = Game.createNpcType(npcId)

-- NPC Properties (from XML)
npcType:name(npcDisplayName)
npcType:nameDescription("a cobra")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(0)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookTypeEx = 1471})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)


local function greetCallback(cid)
	local player = Player(cid)
	if player:getCondition(CONDITION_POISON) then
		player:sendTextMessage(MESSAGE_STATUS_WARNING, "Venture the path of decay!")
		player:getPosition():sendMagicEffect(CONST_ME_TELEPORT)
		player:teleportTo(Position(33396, 32836, 14))
		player:getPosition():sendMagicEffect(CONST_ME_TELEPORT)
		return false
	else
		npcHandler:say("Begone! Hissssss! You bear not the mark of the cobra!", cid)
		return false
	end
	return true
end

npcHandler:setCallback(CALLBACK_GREET, greetCallback)


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
