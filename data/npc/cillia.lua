-- Cillia - Converted from XML to Lua NpcType
-- Original XML: data/npc/Cillia.xml
-- Original Script: data/npc/scripts/Cillia.lua

local npcName = "Cillia"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a cillia")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(2000)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 137, lookHead = 115, lookBody = 94, lookLegs = 78, lookFeet = 114})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)


local function creatureSayCallback(cid, type, msg)
	if not npcHandler:isFocused(cid) then
		return false
	end

	if msgcontains(msg, 'yes') then
		local player = Player(cid)
		if not player:removeMoneyNpc(50) then
			npcHandler:say('The exhibition is not for free. You have to pay 50 Gold to get in. Next please!', cid)
			return true
		end

		npcHandler:say('And here we go!', cid)
		player:getPosition():sendMagicEffect(CONST_ME_TELEPORT)
		local exhibitionPosition = Position(32390, 32195, 8)
		player:teleportTo(exhibitionPosition)
		exhibitionPosition:sendMagicEffect(CONST_ME_TELEPORT)
	else
		npcHandler:say('Then not.', cid)
	end
	npcHandler:releaseFocus(cid)
	npcHandler:resetNpc(cid)
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
