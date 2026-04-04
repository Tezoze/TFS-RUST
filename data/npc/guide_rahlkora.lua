-- Guide Rahlkora - Converted from XML to Lua NpcType
-- Original XML: data/npc/Guide Rahlkora.xml
-- Original Script: data/npc/scripts/Guide Rahlkora.lua

local npcName = "Guide Rahlkora"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a guide rahlkora")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(2000)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 150, lookHead = 79, lookBody = 97, lookLegs = 78, lookFeet = 96})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)


local voices = {
	{ text = 'Need some help finding your way through Ankrahmun? Let me assist you.' },
	{ text = 'Free escort to the depot for newcomers!' },
	{ text = 'Need to know something about the status of this world? Let me answer your questions.' },
	{ text = 'Hello, is this your first visit to Ankrahmun? I can show you around a little.' },
	{ text = 'Talk to me if you need directions.' }
}

npcHandler:addModule(VoiceModule:new(voices))

local configMarks = {
	{mark = "shops", position = Position(33130, 32815, 7), markId = MAPMARK_BAG, description = "Shops"},
	{mark = "depot", position = Position(33126, 32841, 7), markId = MAPMARK_LOCK, description = "Depot"},
	{mark = "temple", position = Position(33195, 32852, 7), markId = MAPMARK_TEMPLE, description = "Temple"}
}

local function creatureSayCallback(cid, type, msg)
	if not npcHandler:isFocused(cid) then
		return false
	end
	local player = Player(cid)
	if isInArray({"map", "marks"}, msg) then
		npcHandler:say("Would you like me to mark locations like - for example - the depot, bank and shops on your map?", cid)
		npcHandler.topic[cid] = 1
	elseif msgcontains(msg, "mission") then
		-- Check if player has already completed and claimed reward
		if player:getStorageValue(Storage.PilgrimageOfAshes.RewardClaimed) == 1 then
			-- Don't respond to mission after completion
			return true
		elseif player:getStorageValue(Storage.PilgrimageOfAshes.Questline) == 6 then
			if player:getLevel() < 25 then
				npcHandler:say({
					"I'm glad to see you've safely returned from your pilgrimage and have all five blessings intact. That means you are fully protected from item loss now and will suffer from a much lighter death penalty, should you die. ...",
					"As reward for your effort, I hand you part of your spent gold back, which can be desperately needed if you've just started your career as an adventurer."
				}, cid)
				player:addItem(2152, 20) -- 20 platinum coins
			else
				npcHandler:say({
					"I'm glad to see you've safely returned from your pilgrimage and have all five blessings intact. That means you are fully protected from item loss now and will suffer from a much lighter death penalty, should you die. ...",
					"As I already granted a discount on the blessings, I won't give you a further reward. This is reserved for our young adventurers only."
				}, cid)
			end
			player:setStorageValue(Storage.PilgrimageOfAshes.RewardClaimed, 1)
			npcHandler:say({
				"If you should repeat the pilgrimage on your own, you will have to pay the full blessing price. I hope this journey was valuable and informative for you. Take care on your adventure!"
			}, cid)
		elseif player:getStorageValue(Storage.PilgrimageOfAshes.Questline) == -1 then
			npcHandler:say("Looking at you, young wanderer, makes me want to help you protect yourself in case you encounter a tragic fate. Death strikes quickly sometimes. I can send you on the guided Pilgrimage of Ashes. What do you say?", cid)
			npcHandler.topic[cid] = 2
		else
			npcHandler:say("You have already started the Pilgrimage of Ashes quest.", cid)
		end
	elseif msgcontains(msg, "yes") and npcHandler.topic[cid] == 1 then
		npcHandler:say("Here you go.", cid)
		local mark
		for i = 1, #configMarks do
			mark = configMarks[i]
			player:addMapMark(mark.position, mark.markId, mark.description)
		end
		npcHandler.topic[cid] = 0
	elseif msgcontains(msg, "yes") and npcHandler.topic[cid] == 2 then
		npcHandler:say({
			"That's a wise choice. Doing this mission will lead you to five sacred places, where you can buy all blessings for 1000 gold coins less each. So much cheaper than what you will have to pay later on. ...",
			"However note that you need a Premium account to reach the last sacred place - as a free account you can only get four of the five blessings, still cheaper than usual though. So, just to make sure, are you still interested?"
		}, cid)
		npcHandler.topic[cid] = 3
	elseif msgcontains(msg, "yes") and npcHandler.topic[cid] == 3 then
           player:setStorageValue(Storage.PilgrimageOfAshes.Questline, 1)
           player:setStorageValue(Storage.PilgrimageOfAshes.Mission01, 1)
           player:addMapMark(Position(32346, 32362, 7), MAPMARK_GREENSOUTH, "Whiteflower Temple")
		npcHandler:say({
			"The first sacred place you have to travel to is the Whiteflower Temple located to the far south of Thais. Talk to the monk Norf about your mission. ...",
			"I placed a mark on your map, however depending on where you are standing right now you might only see it once you get closer to the temple. Wander to the south of Thais for now, following the white flowers."
		}, cid)
		npcHandler.topic[cid] = 0
	elseif msgcontains(msg, "no") and npcHandler.topic[cid] >= 1 then
		npcHandler:say("Well, nothing wrong about exploring the town on your own. Let me know if you need something!", cid)
		npcHandler.topic[cid] = 0
	end
	return true
end

keywordHandler:addKeyword({'information'}, StdModule.say, {npcHandler = npcHandler, text = 'Currently, I can tell you all about the town, its temple, the bank, shops, spell trainers and the depot.'})
keywordHandler:addKeyword({'temple'}, StdModule.say, {npcHandler = npcHandler, text = 'The temple is in the south-eastern part of town. If you exit the depot to the south and walk east - slightly south-east - you can\'t miss it.'})
keywordHandler:addKeyword({'bank'}, StdModule.say, {npcHandler = npcHandler, text = 'Tesha is in charge of both the bank and the gem store. You can find her on the market pyramid. Just exit the depot on the north side and walk up the next pyramid.'})
keywordHandler:addKeyword({'shops'}, StdModule.say, {npcHandler = npcHandler, text = 'You can buy weapons, armor, tools, gems, magical equipment, furniture, spells and food here.'})
keywordHandler:addKeyword({'depot'}, StdModule.say, {npcHandler = npcHandler, text = 'The depot is a place where you can safely store your belongings. You are also protected against attacks there. I escort newcomers there.'})
keywordHandler:addKeyword({'job'}, StdModule.say, {npcHandler = npcHandler, text = 'I will help you to find your way around Ankrahmun. I can mark important locations on your map and give you some information about the town.'})
keywordHandler:addKeyword({'town'}, StdModule.say, {npcHandler = npcHandler, text = 'Ankrahmun consists almost completely of pyramids. You can walk up and down those pyramids just like you\'d climb up stairs. Most shops are on the upper floors.'})
keywordHandler:addKeyword({'name'}, StdModule.say, {npcHandler = npcHandler, text = 'My name is Rahlkora. I\'m a guide.'})

npcHandler:setMessage(MESSAGE_GREET, "Welcome to Ankrahmun, |PLAYERNAME| This city can be a bit confusing at first. Would you like some information and a {map} guide?")
npcHandler:setMessage(MESSAGE_WALKAWAY, "Good bye.")
npcHandler:setMessage(MESSAGE_FAREWELL, "Good bye and enjoy your stay in Ankrahmun, |PLAYERNAME|")

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
