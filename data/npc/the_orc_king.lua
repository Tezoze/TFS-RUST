-- The Orc King - Converted from XML to Lua NpcType
-- Original XML: data/npc/The Orc King.xml
-- Original Script: data/npc/scripts/The Orc King.lua

local npcName = "The Orc King"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a the orc king")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(2000)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 238})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)


local creatures = { 'Slime', 'Slime', 'Slime', 'Orc Warlord', 'Orc Warlord', 'Orc Leader', 'Orc Leader', 'Orc Leader' }
local function greetCallback(cid)
	local player = Player(cid)
	if player:getStorageValue(Storage.OrcKingGreeting) ~= 1 then
		player:setStorageValue(Storage.OrcKingGreeting, 1)
		for i = 1, #creatures do
			Game.createMonster(creatures[i], Npc():getPosition())
		end
		npcHandler:say('Arrrrgh! A dirty paleskin! To me my children! Kill them my guards!', TALKTYPE_SAY)
		return false
	else
		npcHandler:setMessage(MESSAGE_GREET, 'Harrrrk! You think you are strong now? You shall never escape my wrath! I am immortal!')
	end
	return true
end

local function creatureSayCallback(cid, type, msg)
	if not npcHandler:isFocused(cid) then
		return false
	end

	local player = Player(cid)
	local efreet, marid = player:getStorageValue(Storage.DjinnWar.EfreetFaction.Mission03), player:getStorageValue(Storage.DjinnWar.MaridFaction.Mission03)
	-- Mission 3 - Orc Fortress
	if msgcontains(msg, 'lamp') then
		if efreet == 1 or marid == 1 then
			if player:getStorageValue(Storage.DjinnWar.ReceivedLamp) ~= 1 then
				npcHandler:say({
					'I can sense your evil intentions to imprison a djinn! You are longing for the lamp, which I still possess. ...',
					'Who do you want to trap in this cursed lamp?'
				}, cid)
				npcHandler.topic[cid] = 1
			else
				npcHandler:say('For eons he was trapped in an enchanted lamp by some ancient race. Now he\'s free to roam the world again. Although he cheated me I appreciate what he and his brethren will do to this world, now it\'s the time of the Djinn again!', cid)
			end
		end

	elseif msgcontains(msg, 'cookie') then
		if player:getStorageValue(Storage.WhatAFoolishQuest.Questline) == 31 and player:getStorageValue(Storage.WhatAFoolishQuest.CookieDelivery.OrcKing) ~= 1 then
			npcHandler:say('You bring me a stinking cookie???', cid)
			npcHandler.topic[cid] = 2
		end

	-- Mission 3 - Orc Fortress
	elseif npcHandler.topic[cid] == 1 then
		if msgcontains(msg, 'malor') then
			if efreet == 1 then
				player:setStorageValue(Storage.DjinnWar.EfreetFaction.DoorToLamp, 1)

			elseif marid == 1 then
				player:setStorageValue(Storage.DjinnWar.MaridFaction.DoorToLamp, 1)
			end

			player:setStorageValue(Storage.DjinnWar.ReceivedLamp, 1)
			player:addItem(2344, 1)
			npcHandler:say('I was waiting for this day! Take the lamp and let Malor feel my wrath!', cid)
		else
			npcHandler:say('I don\'t know your enemy, paleskin! Begone!', cid)
		end
		npcHandler.topic[cid] = 0

	elseif npcHandler.topic[cid] == 2 then
		if msgcontains(msg, 'yes') then
			if not player:removeItem(8111, 1) then
				npcHandler:say('You have no cookie that I\'d like.', cid)
				npcHandler.topic[cid] = 0
				return true
			end

			player:setStorageValue(Storage.WhatAFoolishQuest.CookieDelivery.OrcKing, 1)
			if player:getCookiesDelivered() == 10 then
				player:addAchievement('Allow Cookies?')
			end

			Npc():getPosition():sendMagicEffect(CONST_ME_GIFT_WRAPS)
			npcHandler:say('Well, I hope it stinks a lot. I like stinking cookies best ... BY MY THOUSAND SONS! YOU ARE SO DEAD HUMAN! DEAD!', cid)
			npcHandler:releaseFocus(cid)
			npcHandler:resetNpc(cid)

		elseif msgcontains(msg, 'no') then
			npcHandler:say('I see.', cid)
			npcHandler.topic[cid] = 0
		end
	end
	return true
end

keywordHandler:addKeyword({'immortal'}, StdModule.say, {npcHandler = npcHandler, text = 'I am Charkahn the Slayer! The immortal father of the {orcs} and master of this {hive}.'})
keywordHandler:addKeyword({'orcs'}, StdModule.say, {npcHandler = npcHandler, text = 'The orcs are the bearers of Blogs rage. This makes us the ultimate fighters and the most powerful of all races.'})
keywordHandler:addKeyword({'divine'}, StdModule.say, {npcHandler = npcHandler, text = 'The orcs are the bearers of Blogs rage. This makes us the ultimate fighters and the most powerful of all races.'})
keywordHandler:addKeyword({'hive'}, StdModule.say, {npcHandler = npcHandler, text = 'I can sense the presence and the feelings of my underlings and {minions}. I embrace the rage of the horde.'})
keywordHandler:addKeyword({'minions'}, StdModule.say, {npcHandler = npcHandler, text = 'The orcish horde of this hive is under my control. I sense their emotions and their needs and provide them with the leadership they need to focus their hate and rage'})
keywordHandler:addKeyword({'hate'}, StdModule.say, {npcHandler = npcHandler, text = 'Hate and rage are the true blessings of Blog, since they are powerful weapons. They give the hive strength. I provide them with direction and focus.'})
keywordHandler:addKeyword({'blog'}, StdModule.say, {npcHandler = npcHandler, text = 'The Raging One blessed us with his burning hate. We are truly his children and therefore {divine}.'})
keywordHandler:addKeyword({'direction'}, StdModule.say, {npcHandler = npcHandler, text = 'To conquer, to destroy and to dominate. Orcs are born to rule the {world}.'})
keywordHandler:addKeyword({'world'}, StdModule.say, {npcHandler = npcHandler, text = 'One day I will rule the world, even when turned into a {slime}.'})
keywordHandler:addKeyword({'slime'}, StdModule.say, {npcHandler = npcHandler, text = 'Pah! Don\'t mock me, mortal! This shape is a curse which the evil {djinn} bestowed upon me!'})
keywordHandler:addKeyword({'djinn'}, StdModule.say, {npcHandler = npcHandler, text = 'This cursed djinn king! I set him free from an enchanted {lamp}, and he {cheated} me!'})
keywordHandler:addKeyword({'cheated'}, StdModule.say, {npcHandler = npcHandler, text = 'Because I freed him he granted me three wishes. He was true to his word in the first two {wishes}.'})
keywordHandler:addKeyword({'wishes'}, StdModule.say, {npcHandler = npcHandler, text = 'He built this fortress over Uldrek\'s grave within a single night. Also, he granted me my second wish and gave me immortality. Test it and try to kill me if you want. Har Har!'})
keywordHandler:addKeyword({'third'}, StdModule.say, {npcHandler = npcHandler, text = 'I wished to father more healthy and fertile children as any orc has ever done. But the djinn cheated me and made me a slime! Then he laughed at me and left for his abandoned fortress in the {Deathwish} Mountains.'})
keywordHandler:addKeyword({'deathwish'}, StdModule.say, {npcHandler = npcHandler, text = 'His ancient fortress on Darama was deserted as the evil Djinn fled this world after his imprisonment. Now the time has come for the evil Djinns to return to their master although this will certainly awaken the {good Djinn} too.'})
keywordHandler:addKeyword({'good djinn'}, StdModule.say, {npcHandler = npcHandler, text = 'I will not share anything more about that topic with you {paleskins}.'})
keywordHandler:addKeyword({'paleskins'}, StdModule.say, {npcHandler = npcHandler, text = 'You are as ugly as maggots, although not quite as as tasty.'})


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
