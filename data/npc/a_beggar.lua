-- A Beggar - Converted from XML to Lua NpcType
-- Original XML: data/npc/A Beggar.xml
-- Original Script: data/npc/scripts/A Beggar.lua

local npcName = "A Beggar"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a a beggar")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(2000)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 153, lookHead = 39, lookBody = 39, lookLegs = 39, lookFeet = 76})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)



function creatureSayCallback(cid, type, msg)
	if(not(npcHandler:isFocused(cid))) then
		return false
	end


	if(msgcontains(msg, "want")) then
		if(getPlayerStorageValue(cid, 10050) == 1) then
			npcHandler:say("The guys from the magistrate sent you here, didn't they?", cid)
			npcHandler.topic[cid] = 1
		end
	elseif(msgcontains(msg, "yes")) then
		if(npcHandler.topic[cid] == 1) then
			selfSay("Thought so. You'll have to talk to the king though. The beggar king that is. The king does not grant an audience to just everyone. You know how those kings are, don't you? ... ", cid)
			selfSay("However, to get an audience with the king, you'll have to help his subjects a bit. ... ", cid)
			selfSay("His subjects that would be us, the poor, you know? ... ", cid)
			selfSay("So why don't you show your dedication to the poor? Go and help Chavis at the poor house. He's collecting food for people like us. ... ", cid)
			selfSay("If you brought enough of the stuff you'll see that the king will grant you entrance in his {palace}.", cid)
			npcHandler.topic[cid] = 0
			setPlayerStorageValue(cid, 20051, 1) -- quest log mission 1 completada
			setPlayerStorageValue(cid, 20052, 0) -- quest log mission 2
			setPlayerStorageValue(cid, 871241, 1) -- quest log mission 2
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
