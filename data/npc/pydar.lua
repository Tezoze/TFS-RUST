-- Pydar - Converted from XML to Lua NpcType
-- Original XML: data/npc/Pydar.xml
-- Original Script: data/npc/scripts/Pydar.lua

local npcName = "Pydar"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a pydar")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(2000)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 160, lookHead = 95, lookBody = 94, lookLegs = 132, lookFeet = 118})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)


-- Main callback for healing
local function creatureSayCallback(cid, type, msg)
	if not npcHandler:isFocused(cid) then
		return false
	end
	
	local player = Player(cid)
	if not player then
		return false
	end
	
	-- Healing
	if msgcontains(msg, "heal") then
		local healed = false
		
		if player:getCondition(CONDITION_FIRE) then
			npcHandler:say("You are burning. Let me quench those flames.", cid)
			player:removeCondition(CONDITION_FIRE)
			player:getPosition():sendMagicEffect(CONST_ME_MAGIC_GREEN)
			healed = true
		elseif player:getCondition(CONDITION_POISON) then
			npcHandler:say("You are poisoned. Let me soothe your pain.", cid)
			player:removeCondition(CONDITION_POISON)
			player:getPosition():sendMagicEffect(CONST_ME_MAGIC_RED)
			healed = true
		elseif player:getCondition(CONDITION_ENERGY) then
			npcHandler:say("You are electrified, my child. Let me help you to stop trembling.", cid)
			player:removeCondition(CONDITION_ENERGY)
			player:getPosition():sendMagicEffect(CONST_ME_MAGIC_GREEN)
			healed = true
		end
		
		if not healed then
			npcHandler:say("You aren't looking that bad. Sorry, I can't help you. But if you are looking for additional protection you should go on the {pilgrimage} of ashes.", cid)
		end
		
		return true
	end
	
	-- Return false to let keyword handlers process other keywords
	return false
end

npcHandler:setCallback(CALLBACK_MESSAGE_DEFAULT, creatureSayCallback)

-- Spark of the Phoenix (Second Part)
local blessKeyword = keywordHandler:addKeyword({'phoenix'}, function(cid, message, keywords, parameters, node)
	local npcHandler = parameters.npcHandler
	if not npcHandler:isFocused(cid) then
		return false
	end
	
	local player = Player(cid)
	if not player then
		return false
	end
	
	if player:hasBlessing(4) then
		npcHandler:say('You have already received the complete Spark of the Phoenix blessing.', cid)
		return true
	end
	
	if player:getStorageValue(Storage.KawillBlessing) ~= 1 then
		npcHandler:say('This blessing has two parts. You must obtain both parts of the blessing in the correct order (first Kawill, then Pydar). Visit Kawill in the earth temple first.', cid)
		return true
	end
	
	local blessCost = StdModule.calculateRegularBlessingCost(player:getLevel())
	npcHandler:say('The Spark of the Phoenix is given by me and by the great geomancer of the local earth temple. You have received Kawill\'s part. Do you wish to receive my part of the Spark of the Phoenix for ' .. blessCost .. ' gold?', cid)
	npcHandler.topic[cid] = blessCost
	return true
end, {npcHandler = npcHandler})
	blessKeyword:addChildKeyword({'yes'}, function(cid, message, keywords, parameters, node)
		local npcHandler = parameters.npcHandler
		if not npcHandler:isFocused(cid) then
			return false
		end
		
		local player = Player(cid)
		if player:hasBlessing(4) then
			npcHandler:say("Gods have already blessed you with this blessing!", cid)
		elseif player:getStorageValue(Storage.KawillBlessing) ~= 1 then
			npcHandler:say('You must first receive Kawill\'s part of the blessing in the earth temple.', cid)
		else
			local blessCost = npcHandler.topic[cid] or (StdModule.calculateRegularBlessingCost(player:getLevel()) - 1000)
			if not player:removeTotalMoney(blessCost) then
				npcHandler:say("You don't have enough money for blessing.", cid)
			else
				player:addBlessing(4)
				player:setStorageValue(Storage.KawillBlessing, -1) -- Reset storage for future blessing attempts
				npcHandler:say("So receive the spark of the phoenix, pilgrim.", cid)
				player:getPosition():sendMagicEffect(CONST_ME_MAGIC_BLUE)
			end
		end
		npcHandler.topic[cid] = 0
		npcHandler:resetNpc(cid)
		return true
	end, {npcHandler = npcHandler})
	blessKeyword:addChildKeyword({''}, StdModule.say, {npcHandler = npcHandler, text = 'Maybe another time.', reset = true})
keywordHandler:addAliasKeyword({'spark'})

-- Pilgrimage of Ashes Mission
local missionKeyword = keywordHandler:addKeyword({'mission'}, function(cid, message, keywords, parameters, node)
	local npcHandler = parameters.npcHandler
	if not npcHandler:isFocused(cid) then
		return false
	end

	local player = Player(cid)
	if not player then
		return false
	end

	-- Check if player has started the Pilgrimage of Ashes quest
	if player:getStorageValue(Storage.PilgrimageOfAshes.Questline) < 1 then
		npcHandler:say("I sense you are not on the pilgrimage of ashes. You should start this quest with one of the city guides first.", cid)
		return true
	end

	-- Check if player has already completed this blessing
	if player:getStorageValue(Storage.PilgrimageOfAshes.Mission04) >= 3 then
		npcHandler:say("You have already received the Spark of the Phoenix blessing. You should continue your pilgrimage to the next sacred place.", cid)
		return true
	end

	-- Check if player has received Kawill's part first
	if player:getStorageValue(Storage.KawillBlessing) ~= 1 then
		npcHandler:say('You must first receive Kawill\'s part of the blessing in the earth temple.', cid)
		return true
	end

	local blessCost = StdModule.calculateRegularBlessingCost(player:getLevel()) - 1000
	npcHandler:say('Blessed pilgrim seeking the spark of the phoenix, would you like to receive my part of the blessing for ' .. blessCost .. ' gold coins?', cid)
	npcHandler.topic[cid] = 1000  -- Using 1000 as topic identifier for mission blessing
	return true
end, {npcHandler = npcHandler})
	missionKeyword:addChildKeyword({'yes'}, function(cid, message, keywords, parameters, node)
		local npcHandler = parameters.npcHandler
		if not npcHandler:isFocused(cid) then
			return false
		end

		local player = Player(cid)
		local blessCost = npcHandler.topic[cid] or (StdModule.calculateRegularBlessingCost(player:getLevel()) - 1000)

		if player:hasBlessing(4) then
			npcHandler:say("Gods have already blessed you with this blessing!", cid)
		elseif player:getStorageValue(Storage.KawillBlessing) ~= 1 then
			npcHandler:say('You must first receive Kawill\'s part of the blessing.', cid)
		elseif not player:removeTotalMoney(blessCost) then
			npcHandler:say("You don't have enough money for blessing.", cid)
		else
			player:addBlessing(4)
			player:setStorageValue(Storage.PilgrimageOfAshes.Mission04, 3)
			player:setStorageValue(Storage.PilgrimageOfAshes.Questline, 5)
			player:setStorageValue(Storage.PilgrimageOfAshes.Mission05, 1)
			player:setStorageValue(Storage.KawillBlessing, -1) -- Reset storage for future blessing attempts
			player:addMapMark(Position(33322, 31882, 7), MAPMARK_GREENNORTH, "Cormaya")
			npcHandler:say({
				"So receive the mark of the flame and be blessed by the phoenix, pilgrim. This is the fourth of five available blessings. Let me tell you where you will find the last sacred place: ...",
				"Travel to Edron, and from there use the ship to the island of Cormaya. Ask the ferryman Pemaret for a passage to the island of the hermit Eremo. I place a mark on your map. Safe travels!"
			}, cid)
			player:getPosition():sendMagicEffect(CONST_ME_MAGIC_BLUE)
		end
		npcHandler.topic[cid] = 0
		npcHandler:resetNpc(cid)
		return true
	end, {npcHandler = npcHandler})
	missionKeyword:addChildKeyword({''}, StdModule.say, {npcHandler = npcHandler, text = 'Fine. You are free to decline my offer.', reset = true})

-- Basic
keywordHandler:addKeyword({'gods'}, StdModule.say, {npcHandler = npcHandler, text = 'The ways of the gods are imprehensible to mortals. On the other hand, the elements are raw forces and can be understood and tamed.'})
keywordHandler:addKeyword({'job'}, StdModule.say, {npcHandler = npcHandler, text = 'I am the head pyromancer of Kazordoon.'})
keywordHandler:addKeyword({'name'}, StdModule.say, {npcHandler = npcHandler, text = 'My name is Pydar Firefist, Son of Fire, from the Savage Axes.'})
keywordHandler:addKeyword({'quest'}, StdModule.say, {npcHandler = npcHandler, text = 'Ask around. There\'s a lot to do, jawoll.'})
keywordHandler:addKeyword({'tibia'}, StdModule.say, {npcHandler = npcHandler, text = 'That is our world.'})
keywordHandler:addKeyword({'time'}, StdModule.say, {npcHandler = npcHandler, text = 'It\'s the fourth age of the yellow flame.'})
keywordHandler:addKeyword({'monsters'}, StdModule.say, {npcHandler = npcHandler, text = 'May the great flame devour them all!'})
keywordHandler:addKeyword({'life'}, StdModule.say, {npcHandler = npcHandler, text = 'Life feeds on fire and ultimately fire will feed on life.'})
keywordHandler:addKeyword({'excalibug'}, StdModule.say, {npcHandler = npcHandler, text = 'A weapon too powerful to be wielded by mortals. It has to be returned to the fire which gave birth to it.'})
keywordHandler:addKeyword({'ferumbras'}, StdModule.say, {npcHandler = npcHandler, text = 'If he ever dares enter Kazordoon I will gladly dump him into the lava. Tthe sacred flame shall bring justice upon him.'})
keywordHandler:addKeyword({'kazordoon'}, StdModule.say, {npcHandler = npcHandler, text = 'Our city was founded in ancient times. Abandoned by the gods we once fought for, we created a secure haven for our people.'})
keywordHandler:addKeyword({'the big old one'}, StdModule.say, {npcHandler = npcHandler, text = 'This mountain is said to be the oldest in the world. It is the place where fire and earth meet and separate at the same time.'})
keywordHandler:addKeyword({'bezil'}, StdModule.say, {npcHandler = npcHandler, text = 'Bezil and Nezil are buying and selling equipment of all kinds.'})
keywordHandler:addKeyword({'nezil'}, StdModule.say, {npcHandler = npcHandler, text = 'Bezil and Nezil are buying and selling equipment of all kinds.'})
keywordHandler:addKeyword({'duria'}, StdModule.say, {npcHandler = npcHandler, text = 'She is the first knight of Kazordoon. She is responsible for teaching our young warriors how to handle an axe.'})
keywordHandler:addKeyword({'etzel'}, StdModule.say, {npcHandler = npcHandler, text = 'Etzel is a true master of the elements. He is a role-model for our youngsters, jawoll.'})
keywordHandler:addKeyword({'jimbin'}, StdModule.say, {npcHandler = npcHandler, text = 'He and his wife are running the Jolly Axeman tavern.'})
keywordHandler:addKeyword({'kroox'}, StdModule.say, {npcHandler = npcHandler, text = 'He is a smith. If you are looking for exquisite weapons and armour just talk to him.'})
keywordHandler:addKeyword({'maryza'}, StdModule.say, {npcHandler = npcHandler, text = 'She and her husband are running the Jolly Axeman tavern.'})
keywordHandler:addKeyword({'uzgod'}, StdModule.say, {npcHandler = npcHandler, text = 'Uzgod is a weaponsmith just like those in the old legends.'})
keywordHandler:addKeyword({'durin'}, StdModule.say, {npcHandler = npcHandler, text = 'Though we are through with the so-called gods, Durin, the first dwarf to aquire divine powers of his own, is considered a protector of our race.'})
keywordHandler:addKeyword({'fire'}, StdModule.say, {npcHandler = npcHandler, text = 'Unlike the gods, the elements don\'t use mortals as toys, A skilled mind can understand and even control them to some extent.'})
keywordHandler:addKeyword({'keeper'}, StdModule.say, {npcHandler = npcHandler, text = 'The ways of the gods are imprehensible to mortals. On the other hand, the elements are raw forces and can be understood and tamed.'})
keywordHandler:addKeyword({'spiritual'}, StdModule.say, {npcHandler = npcHandler, text = 'You can receive the Spiritual Shielding in the Whiteflower Temple south of Thais.'})
keywordHandler:addKeyword({'suns'}, StdModule.say, {npcHandler = npcHandler, text = 'Ask for the Fire of the Suns in the Suntower near Ab\'Dendriel.'})
keywordHandler:addKeyword({'embrace'}, StdModule.say, {npcHandler = npcHandler, text = 'The druids north of Carlin can provide you with the Embrace of Tibia.'})
keywordHandler:addKeyword({'solitude'}, StdModule.say, {npcHandler = npcHandler, text = 'Talk to the hermit Eremo on the isle of Cormaya about this blessing.'})
-- Healing is now handled in creatureSayCallback above
keywordHandler:addKeyword({'pilgrimage'}, StdModule.say, {npcHandler = npcHandler, text = 'I\'m the head pyromancer of Kazordoon. I guess you are here for healing or looking for a blessing.'})
keywordHandler:addKeyword({'blessing'}, StdModule.say, {npcHandler = npcHandler, text = 'There are five different blessings available in five sacred places. These blessings are: the {spiritual} shielding, the spark of the {phoenix}, the {embrace} of Tibia, the fire of the {suns} and the wisdom of {solitude}.'})
keywordHandler:addKeyword({'pyromancer'}, StdModule.say, {npcHandler = npcHandler, text = 'We are the keepers and shepherds of the elemental force of {fire}.'})

npcHandler:setMessage(MESSAGE_GREET, 'Be greeted |PLAYERNAME|! I can smell the scent of a phoenix on you!')
npcHandler:setMessage(MESSAGE_FAREWELL, 'May the fire in your heart never die, |PLAYERNAME|!')
npcHandler:setMessage(MESSAGE_WALKAWAY, 'May the fire in your heart never die.')


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
