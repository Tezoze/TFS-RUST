-- Miles, The Guard - Converted from XML to Lua NpcType
-- Original XML: data/npc/Miles, The Guard.xml
-- Original Script: data/npc/scripts/Miles, The Guard.lua

local npcName = "Miles, The Guard"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a miles, the guard")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(2000)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 131, lookBody = 19, lookLegs = 19, lookFeet = 19})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)


local function creatureSayCallback(cid, type, msg)
	if not npcHandler:isFocused(cid) then
		return false
	end
	local player = Player(cid)
	if msgcontains(msg, "trouble") and npcHandler.topic[cid] ~= 3 and (player:getStorageValue(Storage.TheInquisition.MilesGuard) or 0) < 1 and (player:getStorageValue(Storage.TheInquisition.Mission01) or 0) ~= -1 then
		npcHandler:say("I'm fine. There's no trouble at all.", cid)
		npcHandler.topic[cid] = 1
	elseif msgcontains(msg, "foresight of authorities") and npcHandler.topic[cid] == 1 then
		npcHandler:say("Well, of course. We live in safety and peace.", cid)
		npcHandler.topic[cid] = 2
	elseif msgcontains(msg, "also for the gods") and npcHandler.topic[cid] == 2 then
		npcHandler:say("I think the gods are looking after us and their hands shield us from evil.", cid)
		npcHandler.topic[cid] = 3
	elseif msgcontains(msg, "trouble") and npcHandler.topic[cid] == 3 then
		npcHandler:say("I think the gods and the government do their best to keep away harm from the citizens.", cid)
		npcHandler.topic[cid] = 0
		if (player:getStorageValue(Storage.TheInquisition.MilesGuard) or 0) < 1 then
			player:setStorageValue(Storage.TheInquisition.MilesGuard, 1)
			player:setStorageValue(Storage.TheInquisition.Mission01, (player:getStorageValue(Storage.TheInquisition.Mission01) or 0) + 1) -- The Inquisition Questlog- "Mission 1: Interrogation"
			player:getPosition():sendMagicEffect(CONST_ME_HOLYAREA)
		end
	end
	return true
end

keywordHandler:addKeyword({'job'}, StdModule.say, {npcHandler = npcHandler, text = "It's my duty to protect the city."})

npcHandler:setMessage(MESSAGE_GREET, "LONG LIVE THE KING!")
npcHandler:setMessage(MESSAGE_FAREWELL, "LONG LIVE THE KING!")
npcHandler:setMessage(MESSAGE_WALKAWAY, "LONG LIVE THE KING!")

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
