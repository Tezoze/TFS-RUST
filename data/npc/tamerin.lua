-- Tamerin - Converted from XML to Lua NpcType
-- Original XML: data/npc/Tamerin.xml
-- Original Script: data/npc/scripts/Tamerin.lua

local npcName = "Tamerin"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a tamerin")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(2000)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 153, lookHead = 58, lookBody = 119, lookLegs = 120, lookFeet = 121, lookAddons = 3})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)


local function greetCallback(cid)
	local player = Player(cid)
	if player:getStorageValue(Storage.InServiceofYalahar.Questline) == 30 then
		npcHandler:setMessage(MESSAGE_GREET, "Have you the {animal cure}?")
	elseif player:getStorageValue(Storage.InServiceofYalahar.Questline) == 31 then
		npcHandler:setMessage(MESSAGE_GREET, "Have you killed {morik}?")
	else
		npcHandler:setMessage(MESSAGE_GREET, "Hello, what brings you here?")
	end
	return true
end

local function creatureSayCallback(cid, type, msg)
	if not npcHandler:isFocused(cid) then
		return false
	end
	local player = Player(cid)
	if msgcontains(msg, "mission") then
		if player:getStorageValue(Storage.InServiceofYalahar.Questline) == 29 then
			npcHandler:say({
				"Why should I do something for another human being? I have been on my own for all those years. Hmm, but actually there is something I could need some assistance with. ... ",
				"If you help me to solve my problems, I will help you with your mission. Do you accept?"
			}, cid)
			npcHandler.topic[cid] = 1
		elseif player:getStorageValue(Storage.InServiceofYalahar.Questline) == 32 then
			npcHandler:say("You have kept your promise. Now, it's time to fulfil my part of the bargain. What kind of animals shall I raise? {Warbeasts} or {cattle}?", cid)
			npcHandler.topic[cid] = 2
		end
	elseif msgcontains(msg, "animal cure") then
		if player:getStorageValue(Storage.InServiceofYalahar.Questline) == 30 and player:removeItem(9734, 1) then
			player:setStorageValue(Storage.InServiceofYalahar.Questline, 31)
			player:setStorageValue(Storage.InServiceofYalahar.MorikSummon, 0)
			player:setStorageValue(Storage.InServiceofYalahar.Mission05, 4) -- StorageValue for Questlog "Mission 05: Food or Fight"
			npcHandler:say("Thank you very much. As I said, as soon as you have helped me to solve both of my problems, we will talk about your mission. Have you killed {morik}?", cid)
			npcHandler.topic[cid] = 0
		else
			npcHandler:say("Come back when you have the cure.", cid)
		end
	elseif msgcontains(msg, "cattle") then
		if npcHandler.topic[cid] == 2 then
			player:setStorageValue(Storage.InServiceofYalahar.TamerinStatus, 1)
			player:setStorageValue(Storage.InServiceofYalahar.Mission05, 6) -- StorageValue for Questlog "Mission 05: Food or Fight"
			npcHandler:say("So be it!", cid)
			npcHandler.topic[cid] = 0
		end
	elseif msgcontains(msg, "warbeast") then
		if npcHandler.topic[cid] == 2 then
			player:setStorageValue(Storage.InServiceofYalahar.TamerinStatus, 2)
			player:setStorageValue(Storage.InServiceofYalahar.Mission05, 7) -- StorageValue for Questlog "Mission 05: Food or Fight"
			npcHandler:say("So be it!", cid)
			npcHandler.topic[cid] = 0
		end
	elseif msgcontains(msg, "morik") then
		if player:getStorageValue(Storage.InServiceofYalahar.Questline) == 31 and player:removeItem(9735, 1) then
			player:setStorageValue(Storage.InServiceofYalahar.Questline, 32)
			player:setStorageValue(Storage.InServiceofYalahar.Mission05, 5) -- StorageValue for Questlog "Mission 05: Food or Fight"
			npcHandler:say("So he finally got what he deserved. As I said, as soon as you have helped me to solve both of my problems, we will talk about your {mission}.", cid)
			npcHandler.topic[cid] = 0
		else
			npcHandler:say("Come back when you got rid with Morik.", cid)
			player:setStorageValue(Storage.InServiceofYalahar.MorikSummon, 0)
		end
	elseif msgcontains(msg, "yes") then
		if npcHandler.topic[cid] == 1 then
			player:setStorageValue(Storage.InServiceofYalahar.Questline, 30)
			player:setStorageValue(Storage.InServiceofYalahar.Mission05, 3) -- StorageValue for Questlog "Mission 05: Food or Fight"
			npcHandler:say("I ask you for two things! For one thing, I need an animal cure and for another thing, I ask you to get rid of the gladiator Morik for me.", cid)
			npcHandler.topic[cid] = 0
		end
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
