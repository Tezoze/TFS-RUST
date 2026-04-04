-- Gnomespector - Converted from XML to Lua NpcType
-- Original XML: data/npc/Gnomespector.xml
-- Original Script: data/npc/scripts/Gnomespector.lua

local npcName = "Gnomespector"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a gnomespector")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(2000)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 493, lookHead = 59, lookBody = 59, lookLegs = 59, lookFeet = 58})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)


local function creatureSayCallback(cid, type, msg)
	if not npcHandler:isFocused(cid) then
		return false
	end

	local player = Player(cid)
	if not player then
		return false
	end

	if msgcontains(msg, "recruit") then
		if player:getStorageValue(Storage.BigfootBurden.QuestLine) == 6 then
			npcHandler:say({
				"Your examination is quite easy. Just step through the green crystal {apparatus} in the south! We will examine you with what we call g-rays. Where g stands for gnome of course ...",
				"Afterwards walk up to Gnomedix for your ear examination."
			}, cid)
			player:setStorageValue(Storage.BigfootBurden.QuestLine, 8)
			npcHandler.topic[cid] = 1
		end
	elseif msgcontains(msg, "apparatus") and npcHandler.topic[cid] == 1 then
		npcHandler:say("Don't be afraid. It won't hurt! Just step in!", cid)
		npcHandler.topic[cid] = 0
	end
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
