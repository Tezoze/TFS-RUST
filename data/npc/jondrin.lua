-- Jondrin - Converted from XML to Lua NpcType
-- Original XML: data/npc/Jondrin.xml
-- Original Script: data/npc/scripts/Jondrin.lua

local npcName = "Jondrin"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a jondrin")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(2000)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 132, lookHead = 78, lookBody = 25, lookLegs = 30, lookFeet = 97})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)



function creatureSayCallback(cid, type, msg)
	if(not(npcHandler:isFocused(cid))) then
		return false
	end


	if(msgcontains(msg, "necrometer")) then
		--if(getPlayerStorageValue(cid, 10050) <= 10) then
			selfSay("A necrometer? Have you any idea how rare and expensive a necrometer is? There is no way I could justify giving a necrometer to an inexperienced adventurer. Hm, although ... if you weren't inexperienced that would be a different matter. ...", cid)
			selfSay("Did you do any measuring task for Doubleday lately?", cid)
			npcHandler.topic[cid] = 1
		--end
			elseif(msgcontains(msg, "yes")) then
		if(npcHandler.topic[cid] == 1) and (getPlayerStorageValue(cid, 10050) == 9) then
			npcHandler:say("Indeed I heard you did a good job out there. <sigh> I guess that means I can hand you one of our necrometers. Handle it with care", cid)
			npcHandler.topic[cid] = 0
		   setPlayerStorageValue(cid, 10050, 10)
		   setPlayerStorageValue(cid, 20059, 1)
		   setPlayerStorageValue(cid, 30050, 0)
		   doPlayerAddItem(cid, 23495,1)

		   else
		   npcHandler:say("You already got the Necrometer.", cid)
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
