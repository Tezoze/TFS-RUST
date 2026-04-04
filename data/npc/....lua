-- ... - Converted from XML to Lua NpcType
-- Original XML: data/npc/....xml
-- Original Script: data/npc/scripts/....lua

local npcName = "..."
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a ...")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(1500)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 294})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)


local function greetCallback(cid)
	if Game.getStorageValue(GlobalStorage.FerumbrasAscendantQuest.DesperateSoul) ~= 1 then
		npcHandler:setMessage(MESSAGE_GREET, "Hello my friend, I saw that you send my soul here.")
		return true
	end
	return false
end

local function creatureSayCallback(cid, type, msg)
	if not npcHandler:isFocused(cid) then
		return false
	end

	local player = Player(cid)
	if msgcontains(msg, "mission") and player:getStorageValue(Storage.FerumbrasAscension.TarbazNotes) >= 2 then
		npcHandler.topic[cid] = 1
		npcHandler:say("Oh, are you talking about {Tevon}?", cid)
	elseif msgcontains(msg, "tevon") and npcHandler.topic[cid] == 1 then
		npcHandler.topic[cid] = 0
		npcHandler:say("Ok, sure, now you may pass the door.", cid)
		player:setStorageValue(Storage.FerumbrasAscension.TarbazDoor, 1)
	end
	return true
end

npcHandler:setCallback(CALLBACK_GREET, greetCallback)
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
