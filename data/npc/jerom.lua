-- Jerom - Converted from XML to Lua NpcType
-- Original XML: data/npc/Jerom.xml
-- Original Script: data/npc/scripts/Jerom.lua

local npcName = "Jerom"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a jerom")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(2000)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 132, lookHead = 76, lookBody = 78, lookLegs = 78, lookFeet = 114, lookAddons = 1})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)
local talkState = {}
function creatureSayCallback(cid, type, msg)
        if(not npcHandler:isFocused(cid)) then
                return false
        end
local talkUser = NPCHANDLER_CONVBEHAVIOR == CONVERSATION_DEFAULT and 0 or cid
local storage = getPlayerStorageValue(cid, 42329)

if msgcontains(msg, 'mission') then
	if storage == 2 then
		npcHandler:say("Ohh, my Family necklace, thank you very much adventurer! Here take {elvenhair rope} as a reward.", cid)
		setPlayerStorageValue(cid, 42329, 3)
		doSendMagicEffect(getCreaturePosition(cid), CONST_ME_HOLYAREA)
		local item3 = doPlayerAddItem(cid, 7731, 1)
		doPlayerRemoveItem(cid, 8584, 1)
	else
		npcHandler:say("Ask Chief Grarkharok first.", cid)
		setPlayerStorageValue(cid, 42329, 0)
	end
	end
return TRUE
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
