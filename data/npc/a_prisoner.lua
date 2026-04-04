-- A Prisoner - Converted from XML to Lua NpcType
-- Original XML: data/npc/A Prisoner.xml
-- Original Script: data/npc/scripts/A Prisoner.lua

local npcName = "A Prisoner"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a a prisoner")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(2000)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 130, lookHead = 81, lookBody = 21, lookLegs = 54, lookFeet = 94})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)


local function creatureSayCallback(cid, type, msg)
	if not npcHandler:isFocused(cid) then
		return false
	end
	local player = Player(cid)
	if msgcontains(msg, "riddle") then
		if player:getStorageValue(Storage.madMageQuest) ~= 1 then
			npcHandler:say("Great riddle, isn´t it? If you can tell me the correct answer, I will give you something. Hehehe!", cid)
			npcHandler.topic[cid] = 1
		end
	elseif msgcontains(msg, "PD-D-KS-P-PD") then
		if npcHandler.topic[cid] == 1 then
			npcHandler:say("Hurray! For that I will give you my key for - hmm - let´s say ... some apples. Interested?", cid)
			npcHandler.topic[cid] = 2
		end
	elseif msgcontains(msg, "yes") then
		if npcHandler.topic[cid] == 2 then
			if player:removeItem(2674, 1) then
				npcHandler:say("Mnjam - excellent apples. Now - about that key. You are sure want it?", cid)
				npcHandler.topic[cid] = 3
			else
				npcHandler:say("Get some more apples first!", cid)
				npcHandler.topic[cid] = 0
			end
		elseif npcHandler.topic[cid] == 3 then
			npcHandler:say("Really, really?", cid)
			npcHandler.topic[cid] = 4
		elseif npcHandler.topic[cid] == 4 then
			npcHandler:say("Really, really, really, really?", cid)
			npcHandler.topic[cid] = 5
		elseif npcHandler.topic[cid] == 5 then
			player:setStorageValue(Storage.madMageQuest, 1)
			npcHandler:say("Then take it and get happy - or die, hehe.", cid)
			local key = player:addItem(2088, 1)
			if key then
				key:setActionId(3666)
			end
			npcHandler.topic[cid] = 0
		end
	elseif msgcontains(msg, "no") then
		npcHandler:say("Then go away!", cid)
	end
	return true
end

npcHandler:setMessage(MESSAGE_WALKAWAY, "Wait! Don't leave! I want to tell you about my surreal numbers.")
npcHandler:setMessage(MESSAGE_FAREWELL, "Next time we should talk about my surreal numbers.")
npcHandler:setMessage(MESSAGE_GREET, "Huh? What? I can see! Wow! A non-mino. Did they {capture} you as well?")

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
