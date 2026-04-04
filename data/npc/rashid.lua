-- Rashid - Converted from XML to Lua NpcType
-- Original XML: data/npc/Rashid.xml
-- Original Script: data/npc/scripts/Rashid.lua

local npcName = "Rashid"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a rashid")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(2000)
npcType:walkRadius(3)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 146, lookHead = 100, lookBody = 100, lookLegs = 119, lookFeet = 115, lookAddons = 2})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Original Lua script content
-- Rashid's daily locations
local rashidLocations = {
	["Monday"] = Position(32209, 31155, 7),    -- Svargrond, Dankwart's tavern
	["Tuesday"] = Position(32302, 32834, 7),   -- Liberty Bay, Lyonel's tavern
	["Wednesday"] = Position(32580, 32752, 7), -- Port Hope, Clyde's tavern
	["Thursday"] = Position(33070, 32876, 6),  -- Ankrahmun, Arito's tavern
	["Friday"] = Position(33242, 32480, 7),    -- Darashia, Miraia's tavern
	["Saturday"] = Position(33166, 31809, 6),  -- Edron, Mirabell's tavern
	["Sunday"] = Position(32328, 31783, 6)     -- Carlin depot, one floor above
}

-- Track the last teleport day to avoid constant teleporting
local lastTeleportDay = nil

local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)


local function getRashidLocation()
	local day = os.date("%A")
	return rashidLocations[day]
end


local function creatureSayCallback(cid, type, msg)
	if(not npcHandler:isFocused(cid)) then
		return false
	end
	local player = Player(cid)

	if msgcontains(msg, "mission") then

		-- Mission 1
		if player:getStorageValue(Storage.TheTravellingTraderQuest.Mission01) < 1 then
			npcHandler:say(
				"Well, you could attempt the mission to become a recognised trader, but it requires a lot of travelling. Are you willing to try?",
				cid
			)
			npcHandler.topic[cid] = 1
			return true
		end

		if player:getStorageValue(Storage.TheTravellingTraderQuest.Mission01) == 1 then
			npcHandler:say("Have you managed to obtain a rare deer trophy for my customer?", cid)
			npcHandler.topic[cid] = 3
			return true
		end

		-- Mission 2
		if player:getStorageValue(Storage.TheTravellingTraderQuest.Mission01) == 2 and
		   player:getStorageValue(Storage.TheTravellingTraderQuest.Mission02) < 1 then
			npcHandler:say(
				"So, my friend, are you willing to proceed to the next mission to become a recognised trader?",
				cid
			)
			npcHandler.topic[cid] = 4
			return true
		end

		if player:getStorageValue(Storage.TheTravellingTraderQuest.Mission02) == 4 then
			npcHandler:say("Did you bring me the package?", cid)
			npcHandler.topic[cid] = 6
			return true
		end

		-- Mission 3
		if player:getStorageValue(Storage.TheTravellingTraderQuest.Mission02) == 5 and
		   player:getStorageValue(Storage.TheTravellingTraderQuest.Mission03) < 1 then
			npcHandler:say(
				"So, my friend, are you willing to proceed to the next mission to become a recognised trader?",
				cid
			)
			npcHandler.topic[cid] = 7
			return true
		end

		if player:getStorageValue(Storage.TheTravellingTraderQuest.Mission03) == 2 then
			npcHandler:say("Have you brought the cheese?", cid)
			npcHandler.topic[cid] = 9
			return true
		end

		-- Mission 4
		if player:getStorageValue(Storage.TheTravellingTraderQuest.Mission03) == 3 and
		   player:getStorageValue(Storage.TheTravellingTraderQuest.Mission04) < 1 then
			npcHandler:say(
				"So, my friend, are you willing to proceed to the next mission to become a recognised trader?",
				cid
			)
			npcHandler.topic[cid] = 10
			return true
		end

		if player:getStorageValue(Storage.TheTravellingTraderQuest.Mission04) == 2 then
			npcHandler:say("Have you brought the vase?", cid)
			npcHandler.topic[cid] = 12
			return true
		end

		-- Mission 5
		if player:getStorageValue(Storage.TheTravellingTraderQuest.Mission04) == 3 and
		   player:getStorageValue(Storage.TheTravellingTraderQuest.Mission05) < 1 then
			npcHandler:say(
				"So, my friend, are you willing to proceed to the next mission to become a recognised trader?",
				cid
			)
			npcHandler.topic[cid] = 13
			return true
		end

		if player:getStorageValue(Storage.TheTravellingTraderQuest.Mission05) == 2 then
			npcHandler:say("Have you brought a cheap but good crimson sword?", cid)
			npcHandler.topic[cid] = 15
			return true
		end

		-- Mission 6
		if player:getStorageValue(Storage.TheTravellingTraderQuest.Mission05) == 3 and
		   player:getStorageValue(Storage.TheTravellingTraderQuest.Mission06) < 1 then
			npcHandler:say(
				"So, my friend, are you willing to proceed to the next mission to become a recognised trader?",
				cid
			)
			npcHandler.topic[cid] = 16
			return true
		end

		if player:getStorageValue(Storage.TheTravellingTraderQuest.Mission06) == 1 then
			npcHandler:say("Have you brought me a gold fish?", cid)
			npcHandler.topic[cid] = 18
			return true
		end

		-- Final reward
		if player:getStorageValue(Storage.TheTravellingTraderQuest.Mission06) == 2 and
		   player:getStorageValue(Storage.TheTravellingTraderQuest.Mission07) ~= 1 then
			npcHandler:say(
				"Ah, right. <ahem> I hereby declare you - one of my recognised traders! Feel free to offer me your wares!",
				cid
			)
			player:setStorageValue(Storage.TheTravellingTraderQuest.Mission07, 1)
			player:addAchievement('Recognised Trader')
			npcHandler.topic[cid] = 0
			return true
		end

		npcHandler:say("You have already completed all my tasks, my friend.", cid)
		return true
	end

	if(msgcontains(msg, "yes")) then
		if(npcHandler.topic[cid] == 1) then
			npcHandler:say({
				"Very good! I need talented people who are able to handle my wares with care, find good offers and the like, so I'm going to test you. ...",
				"First, I'd like to see if you can dig up rare wares. Something like a ... mastermind shield! ...",
				"Haha, just kidding, fooled you there, didn't I? Always control your nerves, that's quite important during bargaining. ...",
				"Okay, all I want from you is one of these rare deer trophies. I have a customer here in Svargrond who ordered one, so I'd like you to deliver it tome while I'm in Svargrond. ...",
				"Everything clear and understood?"
			}, cid)

			npcHandler.topic[cid] = 2
		elseif(npcHandler.topic[cid] == 2) then
			npcHandler:say("Fine. Then get a hold of that deer trophy and bring it to me while I'm in Svargrond. Just ask me about your mission.", cid)
			player:setStorageValue(Storage.TheTravellingTraderQuest.Questline, 1)
			player:setStorageValue(Storage.TheTravellingTraderQuest.Mission01, 1)
			npcHandler.topic[cid] = 0
		elseif(npcHandler.topic[cid] == 3) then
			if player:removeItem(7397, 1) then
				npcHandler:say("Well done! I'll take that from you. <snags it> Come see me another day, I'll be busy for a while now. ", cid)
				player:setStorageValue(Storage.TheTravellingTraderQuest.Mission01, 2)
				npcHandler.topic[cid] = 0
			end
		elseif(npcHandler.topic[cid] == 4) then
			npcHandler:say({
				"Alright, that's good to hear. From you as my trader and deliveryman, I expect more than finding rare items. ...",
				"You also need to be able to transport heavy wares, weaklings won't get far here. I have ordered a special package from Edron. ...",
				"Pick it up from Willard and bring it back to me while I'm in Liberty Bay. Everything clear and understood?"
			}, cid)
			npcHandler.topic[cid] = 5
		elseif(npcHandler.topic[cid] == 5) then
			npcHandler:say("Fine. Then off you go, just ask Willard about the 'package for Rashid'.", cid)
			player:setStorageValue(Storage.TheTravellingTraderQuest.Mission02, 1)
			npcHandler.topic[cid] = 0
		elseif(npcHandler.topic[cid] == 6) then
			if player:removeItem(7503, 1) then
				npcHandler:say("Great. Just place it over there - yes, thanks, that's it. Come see me another day, I'll be busy for a while now. ", cid)
				player:setStorageValue(Storage.TheTravellingTraderQuest.Mission02, 5)
				npcHandler.topic[cid] = 0
			end
		elseif(npcHandler.topic[cid] == 7) then
			npcHandler:say({
				"Well, that's good to hear. From you as my trader and deliveryman, I expect more than carrying heavy packages. ...",
				"You also need to be fast and deliver wares in time. I have ordered a very special cheese wheel made from Darashian milk. ...",
				"Unfortunately, the high temperature in the desert makes it rot really fast, so it must not stay in the sun for too long. ...",
				"I'm also afraid that you might not be able to use ships because of the smell of the cheese. ...",
				"Please get the cheese from Miraia and bring it to me while I'm in Port Hope. Everything clear and understood?"
			}, cid)
			npcHandler.topic[cid] = 8
		elseif(npcHandler.topic[cid] == 8) then
			npcHandler:say("Okay, then please find Miraia in Darashia and ask her about the {'scarab cheese'}.", cid)
			player:setStorageValue(Storage.TheTravellingTraderQuest.Mission03, 1)
			npcHandler.topic[cid] = 0
		elseif(npcHandler.topic[cid] == 9) then
			if player:removeItem(8112, 1) then
				npcHandler:say("Mmmhh, the lovely odeur of scarab cheese! I really can't understand why most people can't stand it. Thanks, well done! ", cid)
				player:setStorageValue(Storage.TheTravellingTraderQuest.Mission03, 3)
				npcHandler.topic[cid] = 0
			end
		elseif(npcHandler.topic[cid] == 10) then
			npcHandler:say({
				"Well, that's good to hear. From you as my trader and deliveryman, I expect more than bringing stinky cheese. ...",
				"I wonder if you are able to deliver goods so fragile they almost break when looked at. ...",
				"I have ordered a special elven vase from Briasol in Ab'Dendriel. Get it from him and don't even touch it, just bring it to me while I'm in Ankrahmun. Everything clear and understood?"
			}, cid)
			npcHandler.topic[cid] = 11
		elseif(npcHandler.topic[cid] == 11) then
			npcHandler:say("Okay, then please find {Briasol} in {Ab'Dendriel} and ask for a {'fine vase'}.", cid)
			player:setStorageValue(Storage.TheTravellingTraderQuest.Mission04, 1)
			player:addMoney(1000)
			npcHandler.topic[cid] = 0
		elseif(npcHandler.topic[cid] == 12) then
			if player:removeItem(7582, 1) then
				npcHandler:say("I'm surprised that you managed to bring this vase without a single crack. That was what I needed to know, thank you. ", cid)
				player:setStorageValue(Storage.TheTravellingTraderQuest.Mission04, 3)
				npcHandler.topic[cid] = 0
			end
		elseif(npcHandler.topic[cid] == 13) then
			npcHandler:say({
				"Fine! There's one more skill that I need to test and which is cruicial for a successful trader. ...",
				"Of course you must be able to haggle, else you won't survive long in this business. To make things as hard as possible for you, I have the perfect trade partner for you. ...",
				"Dwarves are said to be the most stubborn of all traders. Travel to {Kazordoon} and try to get the smith {Uzgod} to sell a {crimson sword} to you. ...",
				"Of course, it has to be cheap. Don't come back with anything more expensive than 400 gold. ...",
				"And the quality must not suffer, of course! Everything clear and understood?",
				"Dwarves are said to be the most stubborn of all traders. Travel to Kazordoon and try to get the smith Uzgod to sell a crimson sword to you. ..."
			}, cid)
			npcHandler.topic[cid] = 14
		elseif(npcHandler.topic[cid] == 14) then
			npcHandler:say("Okay, I'm curious how you will do with {Uzgod}. Good luck!", cid)
			player:setStorageValue(Storage.TheTravellingTraderQuest.Mission05, 1)
			npcHandler.topic[cid] = 0
		elseif(npcHandler.topic[cid] == 15) then
			if player:removeItem(7385, 1) then
				npcHandler:say("Ha! You are clever indeed, well done! I'll take this from you. Come see me tomorrow, I think we two might get into business after all.", cid)
				player:setStorageValue(Storage.TheTravellingTraderQuest.Mission05, 3)
				npcHandler.topic[cid] = 0
			end
		elseif(npcHandler.topic[cid] == 16) then
			npcHandler:say({
				"My friend, it seems you have already learnt a lot about the art of trading. I think you are more than worthy to become a recognised trader. ...",
				"There is just one little favour that I would ask from you... something personal, actually, forgive my boldness. ...",
				"I have always dreamed to have a small pet, one that I could take with me and which wouldn't cause problems. ...",
				"Could you - just maybe - bring me a small goldfish in a bowl? I know that you would be able to get one, wouldn't you?"
			}, cid)
			npcHandler.topic[cid] = 17
		elseif(npcHandler.topic[cid] == 17) then
			npcHandler:say("Thanks so much! I'll be waiting eagerly for your return then.", cid)
			player:setStorageValue(Storage.TheTravellingTraderQuest.Mission06, 1)
			npcHandler.topic[cid] = 0
		elseif(npcHandler.topic[cid] == 18) then
			if player:removeItem(5929, 1) then
				npcHandler:say("Thank you!! Ah, this makes my day! I'll take the rest of the day off to get to know this little guy. Come see me tomorrow, if you like.", cid)
				player:setStorageValue(Storage.TheTravellingTraderQuest.Mission06, 2)
				npcHandler.topic[cid] = 0
			end
		end
	end
	return true
end

keywordHandler:addKeyword({"job"}, StdModule.say, {npcHandler = npcHandler, text = "I am a travelling trader. I don't buy everything, though. And not from everyone, for that matter."})
keywordHandler:addKeyword({"name"}, StdModule.say, {npcHandler = npcHandler, text = "I am Rashid, son of the desert."})
keywordHandler:addKeyword({"offers"}, StdModule.say, {npcHandler = npcHandler, text = "Of course, old friend. You can also browse only armor, legs, shields, helmets, boots, weapons, enchanted weapons, jewelry or miscellaneous stuff."})
keywordHandler:addKeyword({"ab'dendriel"}, StdModule.say, {npcHandler = npcHandler, text = "Elves... I don't really trust them. All this talk about nature and flowers and treehugging... I'm sure there's some wicked scheme behind all this."})
keywordHandler:addKeyword({"desert"}, StdModule.say, {npcHandler = npcHandler, text = "My beloved hometown! Ah, the sweet scent of the desert sands, the perfect shape of the pyramids... stunningly beautiful."})
keywordHandler:addKeyword({"carlin"}, StdModule.say, {npcHandler = npcHandler, text = "I have to go to Carlin once in a while, since the queen wishes to see my exclusive wares in regular intervals."})
keywordHandler:addKeyword({"cormaya"}, StdModule.say, {npcHandler = npcHandler, text = "Cormaya? Not a good place to make business, it's way too far and small."})
keywordHandler:addKeyword({"darashia"}, StdModule.say, {npcHandler = npcHandler, text = "It's not the real thing, but almost as good. The merchants there claim ridiculous prices, which is fine for my own business."})
keywordHandler:addKeyword({"edron"}, StdModule.say, {npcHandler = npcHandler, text = "Ah yes, Edron! Such a lovely and quiet island! I usually make some nice business there."})
keywordHandler:addKeyword({"fibula"}, StdModule.say, {npcHandler = npcHandler, text = "Too few customers there, it's not worth the trip."})
keywordHandler:addKeyword({"greenshore"}, StdModule.say, {npcHandler = npcHandler, text = "Um... I don't think so."})
keywordHandler:addKeyword({"kazordoon"}, StdModule.say, {npcHandler = npcHandler, text = "I don't like being underground much. I also tend to get lost in these labyrinthine dwarven tunnels, so I rather avoid them."})
keywordHandler:addKeyword({"liberty bay"}, StdModule.say, {npcHandler = npcHandler, text = "When you avoid the slums, it's a really pretty city. Almost as pretty as the governor's daughter."})
keywordHandler:addKeyword({"northport"}, StdModule.say, {npcHandler = npcHandler, text = "Um... I don't think so."})
keywordHandler:addKeyword({"port hope"}, StdModule.say, {npcHandler = npcHandler, text = "I like the settlement itself, but I don't set my foot into the jungle. Have you seen the size of these centipedes??"})
keywordHandler:addKeyword({"senja"}, StdModule.say, {npcHandler = npcHandler, text = "Um... I don't think so."})
keywordHandler:addKeyword({"svargrond"}, StdModule.say, {npcHandler = npcHandler, text = "I wish it was a little bit warmer there, but with a good mug of barbarian mead in your tummy everything gets a lot cosier."})
keywordHandler:addKeyword({"thais"}, StdModule.say, {npcHandler = npcHandler, text = "I feel uncomfortable and rather unsafe in Thais, so I don't really travel there."})
keywordHandler:addKeyword({"vega"}, StdModule.say, {npcHandler = npcHandler, text = "Um... I don't think so."})
keywordHandler:addKeyword({"venore"}, StdModule.say, {npcHandler = npcHandler, text = "Although it's the flourishing trade centre of Tibia, I don't like going there. Too much competition for my taste."})
keywordHandler:addKeyword({"time"}, StdModule.say, {npcHandler = npcHandler, text = "It's almost time to journey on."})
keywordHandler:addKeyword({"king"}, StdModule.say, {npcHandler = npcHandler, text = "Kings, queens, emperors and kaliphs... everyone claims to be different and unique, but actually it's the same thing everywhere."})

npcHandler:setMessage(MESSAGE_GREET, "Ah, a customer! Be greeted, |PLAYERNAME|!")
npcHandler:setMessage(MESSAGE_FAREWELL, "Farewell, |PLAYERNAME|, may the winds guide your way.")
npcHandler:setMessage(MESSAGE_WALKAWAY, "Come back soon!")
npcHandler:setMessage(MESSAGE_SENDTRADE, "Take all the time you need to decide what you want!")

local function onTradeRequest(cid)
	if Player(cid):getStorageValue(Storage.TheTravellingTraderQuest.Mission07) ~= 1 then
		npcHandler:say('Sorry, but you do not belong to my exclusive customers. I have to make sure that I can trust in the quality of your wares.', cid)
		return false
	end

	return true
end

npcHandler:setCallback(CALLBACK_ONTRADEREQUEST, onTradeRequest)
npcHandler:setCallback(CALLBACK_MESSAGE_DEFAULT, creatureSayCallback)


-- Shop items (from XML parameters)
local shopItems = {
    {id = 7414, buy = 0, sell = 20000, subType = 0, name = "abyss hammer"},
    {id = 7426, buy = 0, sell = 8000, subType = 0, name = "amber staff"},
    {id = 2142, buy = 0, sell = 200, subType = 0, name = "ancient amulet"},
    {id = 7404, buy = 0, sell = 20000, subType = 0, name = "assassin dagger"},
    {id = 5917, buy = 0, sell = 150, subType = 0, name = "bandana"},
    {id = 3962, buy = 0, sell = 1500, subType = 0, name = "beastslayer axe"},
    {id = 11374, buy = 0, sell = 1500, subType = 0, name = "beetle necklace"},
    {id = 7403, buy = 0, sell = 40000, subType = 0, name = "berserker"},
    {id = 7406, buy = 0, sell = 6000, subType = 0, name = "blacksteel sword"},
    {id = 7429, buy = 0, sell = 40000, subType = 0, name = "blessed sceptre"},
    {id = 2541, buy = 0, sell = 80, subType = 0, name = "bone shield"},
    {id = 3972, buy = 0, sell = 7500, subType = 0, name = "bonelord helmet"},
    {id = 7379, buy = 0, sell = 1500, subType = 0, name = "brutetamer's staff"},
    {id = 2535, buy = 0, sell = 5000, subType = 0, name = "castle shield"},
    {id = 8850, buy = 0, sell = 40000, subType = 0, name = "chain bolter"},
    {id = 7427, buy = 0, sell = 9000, subType = 0, name = "chaos mace"},
    {id = 12630, buy = 0, sell = 50000, subType = 0, name = "cobra crown"},
    {id = 9931, buy = 0, sell = 500, subType = 0, name = "coconut shoes"},
    {id = 8855, buy = 0, sell = 25000, subType = 0, name = "composite hornbow"},
    {id = 7415, buy = 0, sell = 30000, subType = 0, name = "cranial basher"},
    {id = 3982, buy = 0, sell = 1000, subType = 0, name = "crocodile boots"},
    {id = 2445, buy = 0, sell = 12000, subType = 0, name = "crystal mace"},
    {id = 2125, buy = 0, sell = 400, subType = 0, name = "crystal necklace"},
    {id = 2124, buy = 0, sell = 250, subType = 0, name = "crystal ring"},
    {id = 7449, buy = 0, sell = 600, subType = 0, name = "crystal sword"},
    {id = 8878, buy = 0, sell = 16000, subType = 0, name = "crystalline armor"},
    {id = 2439, buy = 0, sell = 110, subType = 0, name = "daramian mace"},
    {id = 2440, buy = 0, sell = 1000, subType = 0, name = "daramian waraxe"},
    {id = 2521, buy = 0, sell = 400, subType = 0, name = "dark shield"},
    {id = 6300, buy = 0, sell = 1000, subType = 0, name = "death ring"},
    {id = 6500, buy = 0, sell = 1000, subType = 0, name = "demonic essence"},
    {id = 2520, buy = 0, sell = 30000, subType = 0, name = "demon shield"},
    {id = 2136, buy = 0, sell = 32000, subType = 0, name = "demonbone amulet"},
    {id = 7382, buy = 0, sell = 36000, subType = 0, name = "demonrage sword"},
    {id = 2462, buy = 0, sell = 1000, subType = 0, name = "devil helmet"},
    {id = 7387, buy = 0, sell = 3000, subType = 0, name = "diamond sceptre"},
    {id = 8885, buy = 0, sell = 55000, subType = 0, name = "divine plate"},
    {id = 2451, buy = 0, sell = 15000, subType = 0, name = "djinn blade"},
    {id = 2110, buy = 0, sell = 200, subType = 0, name = "doll"},
    {id = 2492, buy = 0, sell = 40000, subType = 0, name = "dragon scale mail"},
    {id = 7402, buy = 0, sell = 15000, subType = 0, name = "dragon slayer"},
    {id = 7430, buy = 0, sell = 3000, subType = 0, name = "dragonbone staff"},
    {id = 7419, buy = 0, sell = 10000, subType = 0, name = "dreaded cleaver"},
    {id = 2503, buy = 0, sell = 30000, subType = 0, name = "dwarven armor"},
    {id = 7857, buy = 0, sell = 6000, subType = 0, name = "earth blacksteel sword"},
    {id = 7866, buy = 0, sell = 30000, subType = 0, name = "earth cranial basher"},
    {id = 7865, buy = 0, sell = 12000, subType = 0, name = "earth crystal mace"},
    {id = 7858, buy = 0, sell = 15000, subType = 0, name = "earth dragon slayer"},
    {id = 7862, buy = 0, sell = 6000, subType = 0, name = "earth headchopper"},
    {id = 7861, buy = 0, sell = 30000, subType = 0, name = "earth heroic axe"},
    {id = 7856, buy = 0, sell = 30000, subType = 0, name = "earth mystic blade"},
    {id = 7867, buy = 0, sell = 6000, subType = 0, name = "earth orcish maul"},
    {id = 7855, buy = 0, sell = 25000, subType = 0, name = "earth relic sword"},
    {id = 7863, buy = 0, sell = 12000, subType = 0, name = "earth war axe"},
    {id = 7438, buy = 0, sell = 2000, subType = 0, name = "elvish bow"},
    {id = 2127, buy = 0, sell = 800, subType = 0, name = "emerald bangle"},
    {id = 7872, buy = 0, sell = 6000, subType = 0, name = "energy blacksteel sword"},
    {id = 7881, buy = 0, sell = 30000, subType = 0, name = "energy cranial basher"},
    {id = 7880, buy = 0, sell = 12000, subType = 0, name = "energy crystal mace"},
    {id = 7873, buy = 0, sell = 15000, subType = 0, name = "energy dragon slayer"},
    {id = 7877, buy = 0, sell = 6000, subType = 0, name = "energy headchopper"},
    {id = 7876, buy = 0, sell = 30000, subType = 0, name = "energy heroic axe"},
    {id = 7871, buy = 0, sell = 30000, subType = 0, name = "energy mystic blade"},
    {id = 7882, buy = 0, sell = 6000, subType = 0, name = "energy orcish maul"},
    {id = 7870, buy = 0, sell = 25000, subType = 0, name = "energy relic sword"},
    {id = 7878, buy = 0, sell = 12000, subType = 0, name = "energy war axe"},
    {id = 2438, buy = 0, sell = 8000, subType = 0, name = "epee"},
    {id = 7747, buy = 0, sell = 6000, subType = 0, name = "fiery blacksteel sword"},
    {id = 7756, buy = 0, sell = 30000, subType = 0, name = "fiery cranial basher"},
    {id = 7755, buy = 0, sell = 12000, subType = 0, name = "fiery crystal mace"},
    {id = 7748, buy = 0, sell = 15000, subType = 0, name = "fiery dragon slayer"},
    {id = 7752, buy = 0, sell = 6000, subType = 0, name = "fiery headchopper"},
    {id = 7751, buy = 0, sell = 30000, subType = 0, name = "fiery heroic axe"},
    {id = 7746, buy = 0, sell = 30000, subType = 0, name = "fiery mystic blade"},
    {id = 7757, buy = 0, sell = 6000, subType = 0, name = "fiery orcish maul"},
    {id = 7745, buy = 0, sell = 25000, subType = 0, name = "fiery relic sword"},
    {id = 7753, buy = 0, sell = 12000, subType = 0, name = "fiery war axe"},
    {id = 9929, buy = 0, sell = 1000, subType = 0, name = "flower dress"},
    {id = 9927, buy = 0, sell = 500, subType = 0, name = "flower wreath"},
    {id = 7457, buy = 0, sell = 2000, subType = 0, name = "fur boots"},
    {id = 7432, buy = 0, sell = 1000, subType = 0, name = "furry club"},
    {id = 7888, buy = 0, sell = 1500, subType = 0, name = "glacier amulet"},
    {id = 7896, buy = 0, sell = 11000, subType = 0, name = "glacier kilt"},
    {id = 7902, buy = 0, sell = 2500, subType = 0, name = "glacier mask"},
    {id = 7897, buy = 0, sell = 11000, subType = 0, name = "glacier robe"},
    {id = 7892, buy = 0, sell = 2500, subType = 0, name = "glacier shoes"},
    {id = 2179, buy = 0, sell = 8000, subType = 0, name = "gold ring"},
    {id = 2466, buy = 0, sell = 20000, subType = 0, name = "golden armor"},
    {id = 2470, buy = 0, sell = 30000, subType = 0, name = "golden legs"},
    {id = 2533, buy = 0, sell = 3000, subType = 0, name = "griffin shield"},
    {id = 2427, buy = 0, sell = 11000, subType = 0, name = "guardian halberd"},
    {id = 2444, buy = 0, sell = 30000, subType = 0, name = "hammer of wrath"},
    {id = 7380, buy = 0, sell = 6000, subType = 0, name = "headchopper"},
    {id = 2452, buy = 0, sell = 50000, subType = 0, name = "heavy mace"},
    {id = 2442, buy = 0, sell = 90, subType = 0, name = "heavy machete"},
    {id = 7389, buy = 0, sell = 30000, subType = 0, name = "heroic axe"},
    {id = 8873, buy = 0, sell = 3000, subType = 0, name = "hibiscus dress"},
    {id = 7766, buy = 0, sell = 6000, subType = 0, name = "icy blacksteel sword"},
    {id = 7775, buy = 0, sell = 30000, subType = 0, name = "icy cranial basher"},
    {id = 7774, buy = 0, sell = 12000, subType = 0, name = "icy crystal mace"},
    {id = 7767, buy = 0, sell = 15000, subType = 0, name = "icy dragon slayer"},
    {id = 7771, buy = 0, sell = 6000, subType = 0, name = "icy headchopper"},
    {id = 7770, buy = 0, sell = 30000, subType = 0, name = "icy heroic axe"},
    {id = 7765, buy = 0, sell = 30000, subType = 0, name = "icy mystic blade"},
    {id = 7776, buy = 0, sell = 6000, subType = 0, name = "icy orcish maul"},
    {id = 7764, buy = 0, sell = 25000, subType = 0, name = "icy relic sword"},
    {id = 7772, buy = 0, sell = 12000, subType = 0, name = "icy war axe"},
    {id = 7422, buy = 0, sell = 25000, subType = 0, name = "jade hammer"},
    {id = 7461, buy = 0, sell = 200, subType = 0, name = "krimhorn helmet"},
    {id = 8877, buy = 0, sell = 16000, subType = 0, name = "lavos armor"},
    {id = 9928, buy = 0, sell = 500, subType = 0, name = "leaf legs"},
    {id = 3968, buy = 0, sell = 1000, subType = 0, name = "leopard armor"},
    {id = 10220, buy = 0, sell = 3000, subType = 0, name = "leviathan's amulet"},
    {id = 5710, buy = 0, sell = 300, subType = 0, name = "light shovel"},
    {id = 7893, buy = 0, sell = 2500, subType = 0, name = "lightning boots"},
    {id = 7901, buy = 0, sell = 2500, subType = 0, name = "lightning headband"},
    {id = 7895, buy = 0, sell = 11000, subType = 0, name = "lightning legs"},
    {id = 7889, buy = 0, sell = 1500, subType = 0, name = "lightning pendant"},
    {id = 7898, buy = 0, sell = 11000, subType = 0, name = "lightning robe"},
    {id = 7424, buy = 0, sell = 5000, subType = 0, name = "lunar staff"},
    {id = 2472, buy = 0, sell = 90000, subType = 0, name = "magic plate armor"},
    {id = 7890, buy = 0, sell = 1500, subType = 0, name = "magma amulet"},
    {id = 7891, buy = 0, sell = 2500, subType = 0, name = "magma boots"},
    {id = 7899, buy = 0, sell = 11000, subType = 0, name = "magma coat"},
    {id = 7894, buy = 0, sell = 11000, subType = 0, name = "magma legs"},
    {id = 7900, buy = 0, sell = 2500, subType = 0, name = "magma monocle"},
    {id = 7463, buy = 0, sell = 6000, subType = 0, name = "mammoth fur cape"},
    {id = 7464, buy = 0, sell = 850, subType = 0, name = "mammoth fur shorts"},
    {id = 7381, buy = 0, sell = 300, subType = 0, name = "mammoth whopper"},
    {id = 2514, buy = 0, sell = 50000, subType = 0, name = "mastermind shield"},
    {id = 2536, buy = 0, sell = 9000, subType = 0, name = "medusa shield"},
    {id = 7386, buy = 0, sell = 12000, subType = 0, name = "mercenary sword"},
    {id = 2113, buy = 0, sell = 1000, subType = 0, name = "model ship"},
    {id = 7384, buy = 0, sell = 30000, subType = 0, name = "mystic blade"},
    {id = 2426, buy = 0, sell = 2000, subType = 0, name = "naginata"},
    {id = 7418, buy = 0, sell = 35000, subType = 0, name = "nightmare blade"},
    {id = 7456, buy = 0, sell = 10000, subType = 0, name = "noble axe"},
    {id = 7460, buy = 0, sell = 1500, subType = 0, name = "norse shield"},
    {id = 7392, buy = 0, sell = 6000, subType = 0, name = "orcish maul"},
    {id = 8891, buy = 0, sell = 15000, subType = 0, name = "paladin armor"},
    {id = 2641, buy = 0, sell = 2000, subType = 0, name = "patched boots"},
    {id = 2446, buy = 0, sell = 23000, subType = 0, name = "pharaoh sword"},
    {id = 5462, buy = 0, sell = 3000, subType = 0, name = "pirate boots"},
    {id = 6096, buy = 0, sell = 1000, subType = 0, name = "pirate hat"},
    {id = 5918, buy = 0, sell = 200, subType = 0, name = "pirate knee breeches"},
    {id = 6095, buy = 0, sell = 500, subType = 0, name = "pirate shirt"},
    {id = 5810, buy = 0, sell = 500, subType = 0, name = "pirate voodoo doll"},
    {id = 2171, buy = 0, sell = 2500, subType = 0, name = "platinum amulet"},
    {id = 7462, buy = 0, sell = 400, subType = 0, name = "ragnir helmet"},
    {id = 7383, buy = 0, sell = 25000, subType = 0, name = "relic sword"},
    {id = 2123, buy = 0, sell = 30000, subType = 0, name = "ring of the sky"},
    {id = 7434, buy = 0, sell = 40000, subType = 0, name = "royal axe"},
    {id = 2133, buy = 0, sell = 2000, subType = 0, name = "ruby necklace"},
    {id = 6553, buy = 0, sell = 45000, subType = 0, name = "ruthless axe"},
    {id = 10219, buy = 0, sell = 3000, subType = 0, name = "sacred tree amulet"},
    {id = 7437, buy = 0, sell = 7000, subType = 0, name = "sapphire hammer"},
    {id = 2135, buy = 0, sell = 200, subType = 0, name = "scarab amulet"},
    {id = 2540, buy = 0, sell = 2000, subType = 0, name = "scarab shield"},
    {id = 10221, buy = 0, sell = 3000, subType = 0, name = "shockwave amulet"},
    {id = 2134, buy = 0, sell = 150, subType = 0, name = "silver brooch"},
    {id = 2402, buy = 0, sell = 500, subType = 0, name = "silver dagger"},
    {id = 5741, buy = 0, sell = 40000, subType = 0, name = "skull helmet"},
    {id = 8889, buy = 0, sell = 18000, subType = 0, name = "skullcracker armor"},
    {id = 7452, buy = 0, sell = 5000, subType = 0, name = "spiked squelcher"},
    {id = 2645, buy = 0, sell = 30000, subType = 0, name = "steel boots"},
    {id = 8880, buy = 0, sell = 16000, subType = 0, name = "swamplair armor"},
    {id = 7425, buy = 0, sell = 500, subType = 0, name = "taurus mace"},
    {id = 2542, buy = 0, sell = 35000, subType = 0, name = "tempest shield"},
    {id = 7887, buy = 0, sell = 1500, subType = 0, name = "terra amulet"},
    {id = 7886, buy = 0, sell = 2500, subType = 0, name = "terra boots"},
    {id = 7903, buy = 0, sell = 2500, subType = 0, name = "terra hood"},
    {id = 7885, buy = 0, sell = 11000, subType = 0, name = "terra legs"},
    {id = 7884, buy = 0, sell = 11000, subType = 0, name = "terra mantle"},
    {id = 7390, buy = 0, sell = 40000, subType = 0, name = "the justice seeker"},
    {id = 6131, buy = 0, sell = 150, subType = 0, name = "tortoise shield"},
    {id = 7388, buy = 0, sell = 30000, subType = 0, name = "vile axe"},
    {id = 3955, buy = 0, sell = 400, subType = 0, name = "voodoo doll"},
    {id = 2454, buy = 0, sell = 12000, subType = 0, name = "war axe"},
    {id = 2079, buy = 0, sell = 8000, subType = 0, name = "war horn"},
    {id = 10570, buy = 0, sell = 5000, subType = 0, name = "witch hat"},
    {id = 7408, buy = 0, sell = 1500, subType = 0, name = "wyvern fang"},
}

-- Helper function to find shop item by id and subType (for fluid containers)
local function getShopItem(itemId, subType, isBuying)
    local itemType = ItemType(itemId)
    if itemType:isFluidContainer() then
        for _, item in ipairs(shopItems) do
            if item.id == itemId and item.subType == subType then
                return item
            end
        end
    end
    -- For non-fluid items, find the entry that matches the operation
    for _, item in ipairs(shopItems) do
        if item.id == itemId then
            if isBuying and item.buy > 0 then
                return item
            elseif not isBuying and item.sell > 0 then
                return item
            end
        end
    end
    -- Fallback to first match
    for _, item in ipairs(shopItems) do
        if item.id == itemId then
            return item
        end
    end
    return nil
end

local function openTradeWindow(cid, message, keywords, parameters, node)
    if not npcHandler:isFocused(cid) then return false end
    local player = Player(cid)
    if not player then return false end
    
    -- Check if player has completed the Travelling Trader quest
    if player:getStorageValue(Storage.TheTravellingTraderQuest.Mission07) ~= 1 then
        npcHandler:say('Sorry, but you do not belong to my exclusive customers. I have to make sure that I can trust in the quality of your wares.', cid)
        return false
    end
    
    local npc = Npc(getNpcCid())
    local shopList = {}
    for _, item in ipairs(shopItems) do
        table.insert(shopList, {id = item.id, buy = item.buy, sell = item.sell, subType = item.subType or 0, name = item.name})
    end
    npc:openShopWindow(player, shopList, function() return true end, function() return true end)
    npcHandler:say('Take all the time you need to browse my wares.', cid)
    return true
end
keywordHandler:addKeyword({'trade'}, openTradeWindow, {npcHandler = npcHandler})


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

        -- Rashid movement system - only teleport once per day
        local npc = Npc()
        if npc then
            local currentDay = os.date("%A")

            -- Only teleport if it's a new day or first time
            if lastTeleportDay ~= currentDay then
                local targetLocation = rashidLocations[currentDay]

                -- Only teleport if Rashid is not already at the correct location
                local currentPos = npc:getPosition()
                if currentPos.x ~= targetLocation.x or currentPos.y ~= targetLocation.y or currentPos.z ~= targetLocation.z then
                    npc:teleportTo(targetLocation)
                end

                -- Update the last teleport day
                lastTeleportDay = currentDay
            end
        end
end)

npcType:eventType(NPCS_EVENT_CLOSECHANNEL)
npcType:onCloseChannel(function(npc, creature)
    setCurrentNpc(npc)
    npcHandler:onPlayerCloseChannel(creature)
end)

npcType:eventType(NPCS_EVENT_SELLITEM)
npcType:onSellItem(function(npc, player, itemId, subType, amount, ignoreEquipped)
    local shopItem = getShopItem(itemId, subType, false)
    if not shopItem or shopItem.sell <= 0 then return false end
    local totalPrice = amount * shopItem.sell
    local itemName = shopItem.name or ItemType(itemId):getName()
    
    local itemSubType = -1
    if ItemType(itemId):isFluidContainer() then
        itemSubType = subType
    end
    
    if doPlayerSellItem(player:getId(), itemId, amount, totalPrice, itemSubType, ignoreEquipped) then
        player:sendTextMessage(MESSAGE_INFO_DESCR, "Sold " .. amount .. "x " .. shopItem.name .. " for " .. (amount * shopItem.sell) .. " gold.")
        return true
    end
    player:sendCancelMessage("You do not have this object.")
    return false
end)

npcHandler:addModule(FocusModule:new())
npcType:register()
