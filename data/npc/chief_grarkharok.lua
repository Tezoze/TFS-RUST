-- Chief Grarkharok - Converted from XML to Lua NpcType
-- Original XML: data/npc/Chief Grarkharok.xml
-- Original Script: data/npc/scripts/Chief Grarkharok.lua

local npcName = "Chief Grarkharok"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a chief grarkharok")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(2000)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 281})
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
	if storage == 0 then
		npcHandler:say("Hrhrhrhr! Me no fear of human! Me Chief Grarkharok!!", cid)
		setPlayerStorageValue(cid, 42329, 1)
	else
		npcHandler:say("Hrhrhrhr!", cid)
	end
elseif msgcontains(msg, 'kill you') then	
	if storage == 1 then
		npcHandler:say("Hrhrhr, take Jerom's family necklace and give it him back Hrhrhr.", cid)
		setPlayerStorageValue(cid, 42329, 2)
		local item3 = doPlayerAddItem(cid, 8584, 1)
	else
		npcHandler:say("Give Jerom's his family necklace and you will get your reward Hrhrhr", cid)
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
