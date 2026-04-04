-- Gate Guardian - Converted from XML to Lua NpcType
-- Original XML: data/npc/Gate Guardian.xml
-- Original Script: data/npc/scripts/Gate Guardian.lua

local npcName = "Gate Guardian"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a gate guardian")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(0)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 334})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)


local function creatureSayCallback(cid, type, msg)
	if not npcHandler:isFocused(cid) then
		return false
	end
	local player = Player(cid)
	if msgcontains(msg, "mission") or msgcontains(msg, "pass") then
		if player:getStorageValue(Storage.WrathoftheEmperor.Questline) == 13 then
			npcHandler:say("You want entranzzze to zzze zzzity?", cid)
			npcHandler.topic[cid] = 1
		end
	elseif msgcontains(msg, "yes") then
		if npcHandler.topic[cid] == 1 then
			npcHandler:say("Mh, zzzezzze paperzzz zzzeem legit, I have orderzzz to let you pazzz. Zzzo be it.", cid)
			player:setStorageValue(Storage.WrathoftheEmperor.Questline, 22)
			player:setStorageValue(Storage.WrathoftheEmperor.Mission05, 2) --Questlog, Wrath of the Emperor "Mission 05: New in Town"
			player:getPosition():sendMagicEffect(CONST_ME_TELEPORT)
			player:teleportTo(Position(33114, 31197, 7), false)
			player:getPosition():sendMagicEffect(CONST_ME_TELEPORT)
			npcHandler.topic[cid] = 0
		end
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
