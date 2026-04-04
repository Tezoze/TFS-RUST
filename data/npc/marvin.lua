-- Marvin - Converted from XML to Lua NpcType
-- Original XML: data/npc/Marvin.xml
-- Original Script: data/npc/scripts/Marvin.lua

local npcName = "Marvin"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a marvin")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(0)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 132, lookHead = 38, lookBody = 109, lookLegs = 14, lookFeet = 57, lookAddons = 2})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)



function creatureSayCallback(cid, type, msg)
	if(not(npcHandler:isFocused(cid))) then
		return false
	end


	if(msgcontains(msg, "funding")) then
		if(getPlayerStorageValue(cid, 10050) == 7) then
			selfSay("So far you earned x votes. Each single vote can be spent on a different topic or you're also able to cast all your votes on one voting. ...", cid)
			selfSay("Well in the topic b you have the possibility to vote for the funding of the {archives}, import of bug {milk} or street {repairs}.", cid)
			npcHandler.topic[cid] = 1
			else selfSay("You cant vote yet.", cid)
		end
			elseif(msgcontains(msg, "archives")) then
		if(npcHandler.topic[cid] == 1) then
			npcHandler:say("How many of your x votes do you want to cast?", cid)
			npcHandler.topic[cid] = 2
		end
	elseif(msgcontains(msg, "1")) then
		if(npcHandler.topic[cid] == 2) then
			npcHandler:say("Did I get that right: You want to cast 1 of your votes on funding the {archives?}", cid)
			npcHandler.topic[cid] = 3
		end
	elseif(msgcontains(msg, "yes")) then
		if(npcHandler.topic[cid] == 3) then
		   setPlayerStorageValue(cid, 10050, 8)
		   setPlayerStorageValue(cid, 20057, 1)
		   setPlayerStorageValue(cid, 20058, 0)
			npcHandler:say("Thanks, you successfully cast your vote. Feel free to continue gathering votes by helping the city! Farewell.", cid)
			npcHandler.topic[cid] = 0
		end

	return true
end
npcHandler:setCallback(CALLBACK_MESSAGE_DEFAULT, creatureSayCallback)
end


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
