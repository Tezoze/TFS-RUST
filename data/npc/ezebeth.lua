-- Ezebeth - Converted from XML to Lua NpcType
-- Original XML: data/npc/Ezebeth.xml
-- Original Script: data/npc/scripts/Ezebeth.lua

local npcName = "Ezebeth"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a ezebeth")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(2000)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 140, lookHead = 96, lookBody = 31, lookLegs = 34, lookFeet = 94, lookAddons = 1})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)



function creatureSayCallback(cid, type, msg)
	if(not(npcHandler:isFocused(cid))) then
		return false
	end


	if(msgcontains(msg, "mission")) then
		if(getPlayerStorageValue(cid, 10050) < 1) then
			npcHandler:say("Well, there is little where we need help beyond the normal tasks you can do for the city. However, there is one thing out of the ordinary where some assistance would be appreciated.", cid)
			npcHandler.topic[cid] = 1

		else npcHandler:say("You already asked for a mission, go to th next.", cid)

		end
	elseif(msgcontains(msg, "assistance")) then
		if(npcHandler.topic[cid] == 1) then
			npcHandler:say(" It's nothing really important, so no one has yet found the time to look it up. It concerns the towns beggars that have started to behave strange lately.", cid)
			npcHandler.topic[cid] = 2
		end


	elseif(msgcontains(msg, "strange")) then
		if(npcHandler.topic[cid] == 2) then
			npcHandler:say("They usually know better than to show up in the streets and harass our citizens, but lately they've grown more bold or desperate or whatever. I ask you to investigate what they are up to. If necessary, you may scare them away a bit.", cid)
			setPlayerStorageValue(cid, 10050, 1)
			setPlayerStorageValue(cid, 20050, 1) -- quest log mission 1
			setPlayerStorageValue(cid, 20051, 0) -- quest log mission 1
			npcHandler.topic[cid] = 0
	    end

		elseif(msgcontains(msg, "outfit")) then
		if(getPlayerStorageValue(cid, 10050) == 18) then
			selfSay("Nice work, take your outfit.", cid)
			setPlayerStorageValue(cid, 10050, 19)
			doPlayerAddOutfit(cid, 610, 1)
            doPlayerAddOutfit(cid, 618, 1)
			else selfSay("You already have the outfit.", cid)
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
