-- Guide Elena - Converted from XML to Lua NpcType
-- Original XML: data/npc/Guide Elena.xml
-- Original Script: data/npc/scripts/Guide Elena.lua

local npcName = "Guide Elena"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a guide elena")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(0)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 140, lookHead = 38, lookBody = 8, lookLegs = 13, lookFeet = 58})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)


local voices = {
	{ text = 'I can inform you about the status of this world, if you\'re interested.' },
	{ text = 'Hello, is this your first visit to Venore? I can show you around a little.' },
	{ text = 'Talk to me if you need directions.' },
	{ text = 'Need some help finding your way through Venore? Let me assist you.' },
	{ text = 'Free escort to the depot for newcomers!' }
}

npcHandler:addModule(VoiceModule:new(voices))

local configMarks = {
	{mark = "shops1", position = Position(32976, 32083, 6), markId = MAPMARK_BAG, description = "Magic Bazar"},
	{mark = "depot", position = Position(32919, 32071, 6), markId = MAPMARK_LOCK, description = "Depot"},
	{mark = "temple", position = Position(32958, 32078, 6), markId = MAPMARK_TEMPLE, description = "Temple"},
	{mark = "shop2", position = Position(32908, 32123, 6), markId = MAPMARK_TEMPLE, description = "Armors and Weapons"},
	{mark = "bank", position = Position(33011, 32053, 6), markId = MAPMARK_TEMPLE, description = "Bank"},
	{mark = "shop3", position = Position(32976, 32045, 6), markId = MAPMARK_TEMPLE, description = "Foods and Plants"}
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

keywordHandler:addKeyword({'information'}, StdModule.say, {npcHandler = npcHandler, text = 'Currently, I can tell you all about the town, its temple, the bank, shops - well, warehouses - spell trainers and the depot, as well as about the adventurer\'s guild, hunting grounds, quests and the world status.'})
keywordHandler:addKeyword({'temple'}, StdModule.say, {npcHandler = npcHandler, text = 'The temple is pretty much in the middle of Venore. If you go south from this harbour, you can\'t miss it.'})
keywordHandler:addKeyword({'bank'}, StdModule.say, {npcHandler = npcHandler, text = 'The bank as well as jewel stores can be found in the House of Wealth, in the north-eastern part of Venore. I can mark it on your map if you want.'})
keywordHandler:addKeyword({'shops'}, StdModule.say, {npcHandler = npcHandler, text = 'You can buy almost everything here! Visit one of our warehouses for weapons, armors, magical equipment, spells, gems, tools, furniture and everything else you can imagine.'})
keywordHandler:addKeyword({'depot'}, StdModule.say, {npcHandler = npcHandler, text = 'The depot is a place where you can safely store your belongings. You are also protected against attacks there. I escort newcomers there.'})
keywordHandler:addKeyword({'job'}, StdModule.say, {npcHandler = npcHandler, text = 'I will help you find your way in the buzzing city of Venore. I can mark important locations on your map and give you some information about the town and the world status.'})
keywordHandler:addKeyword({'town'}, StdModule.say, {npcHandler = npcHandler, text = 'This trading city has been built directly over a swamp and basically stands on stone pillars. We have many large warehouses here. To speak of \'shops\' would be an understatement.'})
keywordHandler:addKeyword({'name'}, StdModule.say, {npcHandler = npcHandler, text = 'I\'m Elena, sweetheart. I love your name, |PLAYERNAME|.'})

npcHandler:setMessage(MESSAGE_GREET, "Welcome to Venore, |PLAYERNAME| Would you like some information and a {map} guide?")
npcHandler:setMessage(MESSAGE_WALKAWAY, "Good bye.")
npcHandler:setMessage(MESSAGE_FAREWELL, "Good bye and enjoy your stay in Venore, |PLAYERNAME|.")

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
