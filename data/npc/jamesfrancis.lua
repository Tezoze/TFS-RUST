-- Jamesfrancis - Converted from XML to Lua NpcType
-- Original XML: data/npc/Jamesfrancis.xml
-- Original Script: data/npc/scripts/Jamesfrancis.lua

local npcName = "Jamesfrancis"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a jamesfrancis")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(2000)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 574, lookHead = 96, lookBody = 57, lookLegs = 38, lookFeet = 76, lookAddons = 3})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)


local playerTopic = {}
local function greetCallback(cid)

	local player = Player(cid)

	if player:getStorageValue(Storage.CultsOfTibia.Minotaurs.Acesso) < 1 then
		npcHandler:setMessage(MESSAGE_GREET, "Gerimor is right. As an expert for minotaurs I am researching these creatures for years. I thought I already knew a lot but the monsters in this cave are {different}. It's a big {mystery}.")
		playerTopic[cid] = 1
	elseif (player:getStorageValue(Storage.CultsOfTibia.Minotaurs.jamesfrancisTask) >= 0 and player:getStorageValue(Storage.CultsOfTibia.Minotaurs.jamesfrancisTask) <= 50)
	and player:getStorageValue(Storage.CultsOfTibia.Minotaurs.Mission) < 3 then
		npcHandler:setMessage(MESSAGE_GREET, "How is your {mission} going?")
		playerTopic[cid] = 5
	elseif player:getStorageValue(Storage.CultsOfTibia.Minotaurs.Mission) == 4 then
		npcHandler:setMessage(MESSAGE_GREET, {"You say the minotaurs were controlled by a very powerful boss they worshipped. This explains why they had so much more power than the normal ones. ...",
		"I'm very thankful. Please go to the Druid of Crunor and tell him what you've seen. He might be interested in that."})
		player:setStorageValue(Storage.CultsOfTibia.Minotaurs.Mission, 5)
		playerTopic[cid] = 10
	end
	npcHandler:addFocus(cid)
	return true
end


local voices = {
	{ text = 'Don\'t enter this area if you are an inexperienced fighter! It would be your end!' }
}
npcHandler:addModule(VoiceModule:new(voices))

local function creatureSayCallback(cid, type, msg)
	if not npcHandler:isFocused(cid) then
		return false
	end

	npcHandler.topic[cid] = playerTopic[cid]
	local player = Player(cid)

	-- Começou a quest
	if msgcontains(msg, "mystery") and npcHandler.topic[cid] == 1 then
			npcHandler:say({"The minotaurs I faced in the cave are much stronger than the normal ones. What I were able to see before I had to flee: all of them seem to belong to a cult worshipping their god. Could you do me a {favour}?"}, cid)
			npcHandler.topic[cid] = 2
			playerTopic[cid] = 2
	elseif msgcontains(msg, "favour") and npcHandler.topic[cid] == 2 then
			npcHandler:say({"I'd like to work in this cave researching the minotaurs. But right now there are too many of hem and what is more, they are too powerful for me. Could you enter the cave and kill at least 50 of these creatures?"}, cid)
			npcHandler.topic[cid] = 3
			playerTopic[cid] = 3
	elseif msgcontains(msg, "yes") and npcHandler.topic[cid] == 3 then
			npcHandler:say({"Very nice. Return to me if you've finished your job."}, cid)
			player:setStorageValue(Storage.CultsOfTibia.Minotaurs.Mission, 2)
			player:setStorageValue(Storage.CultsOfTibia.Minotaurs.jamesfrancisTask, 0)
			player:setStorageValue(Storage.CultsOfTibia.Minotaurs.Acesso, 1)

		if player:getStorageValue(Storage.CultsOfTibia.Questline) < 1 then
			player:setStorageValue(Storage.CultsOfTibia.Questline, 1)
		end

	-- Entregando a quest
	elseif msgcontains(msg, "mission") and npcHandler.topic[cid] == 5 then
		if player:getStorageValue(Storage.CultsOfTibia.Minotaurs.jamesfrancisTask) >= 50 then
			npcHandler:say({"Great job! You have killed at least 50 of these monsters. I give this key to you to open the door to the inner area. Go there and find out what's going on."}, cid)
			player:setStorageValue(Storage.CultsOfTibia.Minotaurs.Mission, 3)
		else
			npcHandler:say({"Come back when you have killed enough minotaurs."}, cid)
		end
	end
	return true



end
npcHandler:setMessage(MESSAGE_WALKAWAY, 'Well, bye then.')

npcHandler:setCallback(CALLBACK_ONADDFOCUS, onAddFocus)
npcHandler:setCallback(CALLBACK_ONRELEASEFOCUS, onReleaseFocus)

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
