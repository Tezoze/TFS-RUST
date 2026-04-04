-- A Strange Fellow - Converted from XML to Lua NpcType
-- Original XML: data/npc/A Strange Fellow.xml
-- Original Script: data/npc/scripts/A Strange Fellow.lua

local npcName = "A Strange Fellow"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a a strange fellow")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(2000)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 128, lookHead = 95, lookBody = 118, lookLegs = 57, lookFeet = 95, lookAddons = 2})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)


local function creatureSayCallback(cid, type, msg)
	if not npcHandler:isFocused(cid) then
		return false
	end
	local player = Player(cid)
	if player:getStorageValue(Storage.postman.Mission03) ~= 1 then
		return true
	end
	if msgcontains(msg, "bill") then
		if	npcHandler.topic[cid] == 6 then
			npcHandler:say("A bill? Oh boy so you are delivering another bill to poor me?", cid)
			npcHandler.topic[cid] = 7
		end
	elseif msgcontains(msg, "yes") then
		if	player:removeItem(2329, 1)	and	npcHandler.topic[cid] == 7 then
			npcHandler:say("Ok, ok, I'll take it. I guess I have no other choice anyways. And now leave me alone in my misery please.", cid)
			player:setStorageValue(Storage.postman.Mission03, 2)
			npcHandler.topic[cid] = 0
		end
	elseif msgcontains(msg, "hat") then
		if	npcHandler.topic[cid] < 1 then
			npcHandler:say("Uh? What do you want?!", cid)
			npcHandler.topic[cid] = 2
		elseif npcHandler.topic[cid] == 2 then
			npcHandler:say("What? My hat?? Theres... nothing special about it!", cid)
			npcHandler.topic[cid] = 3
		elseif npcHandler.topic[cid] == 3 then
			npcHandler:say("Stop bugging me about that hat, do you listen?", cid)
			npcHandler.topic[cid] = 4
		elseif npcHandler.topic[cid] == 4 then
			npcHandler:say("Hey! Don't touch that hat! Leave it alone!!! Don't do this!!!!", cid)
			npcHandler.topic[cid] = 5
		elseif npcHandler.topic[cid] == 5 then
			for i = 1, 5 do
				Game.createMonster("Rabbit", Npc():getPosition())
			end
			npcHandler:say("Noooooo! Argh, ok, ok, I guess I can't deny it anymore, I am David Brassacres, the magnificent, so what do you want?", cid)
			npcHandler.topic[cid] = 6
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
