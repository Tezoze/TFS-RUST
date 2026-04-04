-- Noodles - Converted from XML to Lua NpcType
-- Original XML: data/npc/Noodles.xml
-- Original Script: data/npc/scripts/Noodles.lua

local npcName = "Noodles"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a noodles")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(2000)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 32})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)


local voices = {
	{ text = 'Grrrrrrr.' },
	{ text = '<wiggles>' },
	{ text = '<sniff>' },
	{ text = 'Woof! Woof!' },
	{ text = 'Wooof!' }
}

npcHandler:addModule(VoiceModule:new(voices))

local function creatureSayCallback(cid, type, msg)
	if not npcHandler:isFocused(cid) then
		return false
	end
	local player = Player(cid)
	if msgcontains(msg, "banana skin") then
		if player:getStorageValue(Storage.postman.Mission06) == 7 then
			if player:getItemCount(2219) > 0 then
				npcHandler:say("<sniff><sniff>", cid)
				npcHandler.topic[cid] = 1
			end
		end
	elseif msgcontains(msg, "dirty fur") then
		if player:getStorageValue(Storage.postman.Mission06) == 8 then
			if player:getItemCount(2220) > 0 then
				npcHandler:say("<sniff><sniff>", cid)
				npcHandler.topic[cid] = 2
			end
		end
	elseif msgcontains(msg, "cheese") then
		if player:getStorageValue(Storage.postman.Mission06) == 9 then
			if player:getItemCount(2235) > 0 then
				npcHandler:say("<sniff><sniff>", cid)
				npcHandler.topic[cid] = 3
			end
		end
	elseif msgcontains(msg, "like") then
		if npcHandler.topic[cid] == 1  then
			npcHandler:say("Woof!", cid)
			player:setStorageValue(Storage.postman.Mission06, 8)
			npcHandler.topic[cid] = 0
		elseif npcHandler.topic[cid] == 2 then
			npcHandler:say("Woof!", cid)
			player:setStorageValue(Storage.postman.Mission06, 9)
			npcHandler.topic[cid] = 0
		elseif npcHandler.topic[cid] == 3 then
			npcHandler:say("Meeep! Grrrrr! <spits>", cid)
			player:setStorageValue(Storage.postman.Mission06, 10)
			npcHandler.topic[cid] = 0
		end
	end
	return true
end

npcHandler:setMessage(MESSAGE_GREET, "<sniff> Woof! <sniff>")
npcHandler:setMessage(MESSAGE_FAREWELL, "Woof! <wiggle>")
npcHandler:setMessage(MESSAGE_WALKAWAY, "Woof! <wiggle>")

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
