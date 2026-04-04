-- The Beggar King - Converted from XML to Lua NpcType
-- Original XML: data/npc/The Beggar King.xml
-- Original Script: data/npc/scripts/The Beggar King.lua

local npcName = "The Beggar King"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a the beggar king")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(2000)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 153, lookBody = 114, lookLegs = 94, lookFeet = 78, lookAddons = 3})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)



function creatureSayCallback(cid, type, msg)
	if(not(npcHandler:isFocused(cid))) then
		return false
	end


	if(msgcontains(msg, "mission")) then
		if(getPlayerStorageValue(cid, 10050) <= 2 and getPlayerStorageValue(cid, 871241) == 1) then
			npcHandler:say("So I guess you are the one that the magistrate is sending to look after us, eh? ", cid)
			npcHandler.topic[cid] = 1
			else
			npcHandler:say("You need some quests then come and talk with me again.", cid)
		end
	elseif(msgcontains(msg, "yes")) then
		if(npcHandler.topic[cid] == 1) then
			selfSay("Fine. But the first thing you have to know is that we are not the city's problem. We are just trying to survive. We usually seek shelter in the sewers.", cid)
			selfSay("There we are comparatively warm and safe. At least we were. But recently something has changed. There is something in the sewers. And it is hunting us.", cid)
			npcHandler.topic[cid] = 2
		end
	elseif(msgcontains(msg, "something")) then
		if(npcHandler.topic[cid] == 2) then
			npcHandler:say("Yeah. No one has seen it and lived to tell the tale. People are missing and sometimes there are {traces} of blood or someone heard a scream, but that's all. We have no idea if the killer is a man or a beast, but there is something out there", cid)
			npcHandler.topic[cid] = 3
		end
	elseif(msgcontains(msg, "traces")) then
		if(npcHandler.topic[cid] == 3) then
			npcHandler:say("Some of the more daring of us tried to follow the tracks that were left, but they always lost the trail close to the abandoned sewers, in the east of the sewer system.", cid)
			npcHandler.topic[cid] = 4
		end
	elseif(msgcontains(msg, "abandoned sewers")) then
		if(npcHandler.topic[cid] == 4) then
			selfSay("Some parts of the sewers were abandoned when they were beyond repair due to old age and earthquakes. ...", cid)
			selfSay("That part was never truly well liked. There were rumours that the workers found some ancient structures there and that it was ripe with accidents during the construction. ...", cid)
			selfSay("The city sealed those parts off, and I have no idea how anything could get in or out without the permission of the magistrate. ... ", cid)
			selfSay("But since you are investigating on their behalf, you might work out some agreement with them, if you're mad enough to enter the sewers at all. ... ", cid)
			selfSay("However, you will have to talk to one of the Glooth Brothers who are responsible for the sewer system's maintenance. You'll find them somewhere down there.", cid)
			setPlayerStorageValue(cid, 20052, 1) -- quest log mission 2 completada
			setPlayerStorageValue(cid, 20053, 0) -- quest log mission 2
			setPlayerStorageValue(cid, 10050, 3) -- quest log mission 3
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
