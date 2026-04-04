-- Elliott - Converted from XML to Lua NpcType
-- Original XML: data/npc/Elliott.xml
-- Original Script: data/npc/scripts/Elliott.lua

local npcName = "Elliott"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a elliott")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(2000)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 574, lookHead = 114, lookBody = 114, lookLegs = 114, lookFeet = 114, lookAddons = 3})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)



function creatureSayCallback(cid, type, msg)
	if(not(npcHandler:isFocused(cid))) then
		return false
	end


	if(msgcontains(msg, "abandoned sewers")) then
		if(getPlayerStorageValue(cid, 20062) < 21) then
			selfSay("You want to enter the abandoned sewers? That's rather dangerous and not a good idea, man. That part of the sewers was not sealed off for nothing, you know? ...", cid)
			selfSay("But hey, it's your life, bro. So here's the deal. I'll let you into the abandoned sewers if you help me with our {mission}.", cid)
		elseif(getPlayerStorageValue(cid, 20062) == 21) then
			selfSay("Wow, you already did it, that's fast. I'm used to a more laid-back attitude from most people. It's a shame to risk losing you to some collapsing tunnels, but a deal is a deal. ...", cid)
			selfSay("I hereby grant you the permission to enter the abandoned part of the sewers. Take care, man! ...", cid)
			selfSay("If you find something interesting, come back to talk about the {abandoned sewers}.", cid)
			npcHandler.topic[cid] = 7
            setPlayerStorageValue(cid, 10050, 4)
			setPlayerStorageValue(cid, 20053, 1)
            setPlayerStorageValue(cid, 20054, 0)
            player:setStorageValue(Storage.Oramond.DoorAbandonedSewer, 1)
		elseif(getPlayerStorageValue(cid, 10050) == 5) then
		    npcHandler:say("I'm glad to see you back alive and healthy. Did you find anything interesting that you want to {report}?", cid)
			npcHandler.topic[cid] = 7
	end
	elseif(msgcontains(msg, "mission")) then
		if(npcHandler.topic[cid] == 0) then
			npcHandler:say("The sewers need repair. You in?", cid)
			npcHandler.topic[cid] = 2
		elseif(getPlayerStorageValue(cid, 10050) >= 3) then
			npcHandler:say("Elliott's keeps calling it that. It's just another job! You fixed some broken pipes and stuff? Let me check, {ok}?", cid)
			npcHandler.topic[cid] = 3
		end
	elseif(msgcontains(msg, "yes")) then
		if(npcHandler.topic[cid] == 2) then
			npcHandler:say("Good. Broken pipe and generator pieces, there's smoke evading. That's how you recognise them. See how you can fix them using your hands. Need about, oh, twenty of them at least repaired. Report to me or Jacob", cid)
			npcHandler.topic[cid] = 0
			setPlayerStorageValue(cid, 10062, 1)
			setPlayerStorageValue(cid, 20062, 0)
		end
	elseif(msgcontains(msg, "ok")) then
		if(npcHandler.topic[cid] == 3) then
			npcHandler:say("Good. Thanks, man. That's one vote you got for helping us with this.", cid)
			npcHandler.topic[cid] = 0
			setPlayerStorageValue(cid, 20062, 21)

		end
	elseif(msgcontains(msg, "report")) then
	if(getPlayerStorageValue(cid, 10050) == 5) then
		--if(npcHandler.topic[cid] == 7) then
		    selfSay("He can make more sense of what you found there. His name is Barazbaz. He should be in the magistrate building.", cid)
			selfSay("A sacrificial site? Damn, sounds like some freakish cult or something. Just great. And this ancient structure you talked about that's not part of the sewers? You'd better see the local historian about that, man. ...", cid)
			setPlayerStorageValue(cid, 10050, 6)
			setPlayerStorageValue(cid, 20055, 1)
			setPlayerStorageValue(cid, 20056, 0)
			npcHandler.topic[cid] = 0

			else npcHandler:say("You already reported this mission, go to the next.", cid)

	return true
end
npcHandler:setCallback(CALLBACK_MESSAGE_DEFAULT, creatureSayCallback)
end
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
