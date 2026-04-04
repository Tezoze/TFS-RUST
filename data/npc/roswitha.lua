-- Roswitha - Converted from XML to Lua NpcType
-- Original XML: data/npc/Roswitha.xml
-- Original Script: data/npc/scripts/Roswitha.lua

local npcName = "Roswitha"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a roswitha")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(2000)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 138, lookHead = 114, lookBody = 118, lookLegs = 113, lookFeet = 3})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)



function creatureSayCallback(cid, type, msg)
	if(not(npcHandler:isFocused(cid))) then
		return false
	end


	if(msgcontains(msg, "Harsin")) then
		if(getPlayerStorageValue(cid, 10050) == 13) then
			selfSay("I'm sorry, but Harsin no longer lives here. He ordered a local named Quandon to transport all his stuff somewhere. I don't know where he moved, but Quandon should be able to help you with this information.", cid)
            setPlayerStorageValue(cid, 10050, 14)
		   setPlayerStorageValue(cid, 30052, 1)
		   setPlayerStorageValue(cid, 30053, 0)
		   player:setStorageValue(Storage.DarkTrails.DoorQuandon, 1)
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
