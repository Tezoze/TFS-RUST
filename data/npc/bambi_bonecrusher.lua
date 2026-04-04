-- Bambi Bonecrusher - Converted from XML to Lua NpcType
-- Original XML: data/npc/Bambi Bonecrusher.xml
-- Original Script: data/npc/scripts/Bambi Bonecrusher.lua

local npcName = "Bambi Bonecrusher"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a bambi bonecrusher")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(2000)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 139, lookHead = 96, lookLegs = 68, lookFeet = 95})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)


local condition = Condition(CONDITION_FIRE)
condition:setParameter(CONDITION_PARAM_DELAYED, 1)
condition:addDamage(10, 1000, -10)

local function creatureSayCallback(cid, type, msg)
	if not npcHandler:isFocused(cid) then
		return false
	end
	if isInArray({"fuck", "idiot", "asshole", "ass", "fag", "stupid", "tyrant", "shit", "lunatic"}, msg) then
		npcHandler:say("Take this!", cid)
		local player = Player(cid)
		player:getPosition():sendMagicEffect(CONST_ME_EXPLOSIONAREA)
		player:addCondition(condition)
		npcHandler:releaseFocus(cid)
		npcHandler:resetNpc(cid)
	end
	return true
end

npcHandler:setMessage(MESSAGE_GREET, "LONG LIVE THE QUEEN!")
npcHandler:setMessage(MESSAGE_FAREWELL, "LONG LIVE THE QUEEN!")
npcHandler:setMessage(MESSAGE_WALKAWAY, "LONG LIVE THE QUEEN!")

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
