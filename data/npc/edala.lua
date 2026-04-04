-- Edala - Converted from XML to Lua NpcType
-- Original XML: data/npc/Edala.xml
-- Original Script: data/npc/scripts/Edala.lua

local npcName = "Edala"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a edala")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(2000)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 63})
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
		elseif player:getHealth() < 40 then
			npcHandler:say("You are hurt, my child. I will heal your wounds.", cid)
			local health = player:getHealth()
			if health < 40 then
				player:addHealth(40 - health)
			end
			player:getPosition():sendMagicEffect(CONST_ME_MAGIC_GREEN)
			healed = true
		end
		
		if not healed then
			npcHandler:say("You aren't looking that bad. Sorry, I can't help you. But if you are looking for additional protection you should go on the {pilgrimage} of ashes or get the protection of the {twist of fate} here.", cid)
		end
		
		return true
	end
	
	-- Return false to let keyword handlers process other keywords
	return false
end

npcHandler:setCallback(CALLBACK_MESSAGE_DEFAULT, creatureSayCallback)

local config = {
	[1] = 'Ashari, |PLAYERNAME|. How... nice to see a human taking interest in a beautiful art such as music.',
	[2] = 'Ashari, |PLAYERNAME|... that sound was.. interesting.',
	[3] = 'Ashari, |PLAYERNAME|. You\'ve made some... progress playing the lyre, haven\'t you..? I want to believe you have.',
	[4] = '|PLAYERNAME|. My regular visitor. I certainly... appreciate your efforts to entertain me, but let me assure you, I\'m quite comfortable up here by myself. Alone. In silence.',
	[5] = 'Ashari, |PLAYERNAME|. I\'m starting to feel a little sorry... for your lyre. Being forced to produce such noise must be a tragic fate.',
	[6] = '|PLAYERNAME|! You\'re driving me insane! I beg you, take your lyre away from this sacred and peaceful place.',
	[7] = '|PLAYERNAME|! My ears! I\'d prefer listening to drunken dwarves rambling all day to the sound of your lyre! Please, at least get it tuned. Here, you can have this elvish diapason.'
}

local function greetCallback(cid)
	local player = Player(cid)
	local lyreProgress = player:getStorageValue(Storage.Diapason.Lyre)
	local greetMessage = config[lyreProgress]
	if greetMessage
			and player:getStorageValue(Storage.Diapason.Edala) == 1
			and player:getStorageValue(Storage.Diapason.EdalaTimer) < os.time() then
		player:setStorageValue(Storage.Diapason.Edala, 0)
		player:setStorageValue(Storage.Diapason.EdalaTimer, os.time() + 86400)
		if lyreProgress == 7 then
			player:setStorageValue(Storage.Diapason.Lyre, 8)
			player:addItem(13536, 1)
		end
		npcHandler:setMessage(MESSAGE_GREET, greetMessage)
	else
		npcHandler:setMessage(MESSAGE_GREET, 'Ashari, |PLAYERNAME|.')
	end
	return true
end


-- Fire of the Suns
local blessKeyword = keywordHandler:addKeyword({'suns'}, function(cid, message, keywords, parameters, node)
	local npcHandler = parameters.npcHandler
	if not npcHandler:isFocused(cid) then
		return false
	end
	
	local player = Player(cid)
	if not player then
		return false
	end
	
	local blessCost = StdModule.calculateRegularBlessingCost(player:getLevel())
	npcHandler:say('Would you like to receive that protection for a sacrifice of ' .. blessCost .. ' gold, child?', cid)
	npcHandler.topic[cid] = blessCost
	return true
end, {npcHandler = npcHandler})
	blessKeyword:addChildKeyword({'yes'}, function(cid, message, keywords, parameters, node)
		local npcHandler = parameters.npcHandler
		if not npcHandler:isFocused(cid) then
			return false
		end
		
		local player = Player(cid)
		local blessCost = npcHandler.topic[cid] or (StdModule.calculateRegularBlessingCost(player:getLevel()) - 1000)
		
		if player:hasBlessing(3) then
			npcHandler:say("Gods have already blessed you with this blessing!", cid)
		elseif not player:removeTotalMoney(blessCost) then
			npcHandler:say("You don't have enough money for blessing.", cid)
		else
			player:addBlessing(3)
			npcHandler:say("So receive the fire of the suns, pilgrim.", cid)
			player:getPosition():sendMagicEffect(CONST_ME_MAGIC_BLUE)
		end
		npcHandler.topic[cid] = 0
		npcHandler:resetNpc(cid)
		return true
	end, {npcHandler = npcHandler})
	blessKeyword:addChildKeyword({''}, StdModule.say, {npcHandler = npcHandler, text = 'Fine. You are free to decline my offer.', reset = true})
keywordHandler:addAliasKeyword({'fire'})

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
	if player:getStorageValue(Storage.PilgrimageOfAshes.Mission03) >= 2 then
		npcHandler:say("You have already received the fire of the suns. You should continue your pilgrimage to the next sacred place.", cid)
		return true
	end


	local blessCost = StdModule.calculateRegularBlessingCost(player:getLevel()) - 1000
	npcHandler:say('By the name of Priyla, daughter of the stars, would you like to receive the fire of the suns for ' .. blessCost .. ' gold coins?', cid)
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

		if player:hasBlessing(3) then
			npcHandler:say("Gods have already blessed you with this blessing!", cid)
		elseif not player:removeTotalMoney(blessCost) then
			npcHandler:say("You don't have enough money for blessing.", cid)
		else
			player:addBlessing(3)
			player:setStorageValue(Storage.PilgrimageOfAshes.Mission03, 2)
			player:setStorageValue(Storage.PilgrimageOfAshes.Questline, 4)
			player:setStorageValue(Storage.PilgrimageOfAshes.Mission04, 1)
			player:addMapMark(Position(32644, 31969, 12), MAPMARK_GREENNORTH, "Kazordoon Temples")
			npcHandler:say({
				"So kneel down and receive the warmth of sunfire, pilgrim. This is the third of five available blessings. Let me tell you where you will find the fourth sacred place: ...",
				"Travel to Kazordoon and find the dwarven temples of earth and fire, tended by the priests Pydar and Kawill. Both are needed to cast the spark of the phoenix on you, first earth, then fire. I mark both locations on your map. Safe pilgrimage!"
			}, cid)
			player:getPosition():sendMagicEffect(CONST_ME_MAGIC_BLUE)
		end
		npcHandler.topic[cid] = 0
		npcHandler:resetNpc(cid)
		return true
	end, {npcHandler = npcHandler})
	missionKeyword:addChildKeyword({''}, StdModule.say, {npcHandler = npcHandler, text = 'Fine. You are free to decline my offer.', reset = true})

-- Healing is now handled in creatureSayCallback above

-- Basic
keywordHandler:addKeyword({'pilgrimage'}, StdModule.say, {npcHandler = npcHandler, text = 'Whenever you receive a lethal wound, your vital force is damaged and there is a chance that you lose some of your equipment. With every single of the five {blessings} you have, this damage and chance of loss will be reduced.'})
keywordHandler:addKeyword({'blessings'}, StdModule.say, {npcHandler = npcHandler, text = 'There are five blessings available in five sacred places: the {spiritual} shielding, the spark of the {phoenix}, the {embrace} of Tibia, the fire of the {suns} and the wisdom of {solitude}. Additionally, you can receive the {twist of fate} here.'})
keywordHandler:addKeyword({'spiritual'}, StdModule.say, {npcHandler = npcHandler, text = 'I see you received the spiritual shielding in the whiteflower temple south of Thais.'}, function(player) return player:hasBlessing(1) end)
keywordHandler:addAliasKeyword({'shield'})
keywordHandler:addKeyword({'embrace'}, StdModule.say, {npcHandler = npcHandler, text = 'I can sense that the druids north of Carlin have provided you with the Embrace of Tibia.'}, function(player) return player:hasBlessing(2) end)
keywordHandler:addKeyword({'phoenix'}, StdModule.say, {npcHandler = npcHandler, text = 'I can sense that the spark of the phoenix already was given to you by the dwarven priests of earth and fire in Kazordoon.'}, function(player) return player:hasBlessing(4) end)
keywordHandler:addAliasKeyword({'spark'})
keywordHandler:addKeyword({'solitude'}, StdModule.say, {npcHandler = npcHandler, text = 'I can sense you already talked to the hermit Eremo on the isle of Cormaya and received this blessing.'}, function(player) return player:hasBlessing(5) end)
keywordHandler:addAliasKeyword({'wisdom'})
keywordHandler:addKeyword({'spiritual'}, StdModule.say, {npcHandler = npcHandler, text = 'You can ask for the blessing of spiritual shielding in the whiteflower temple south of Thais.'})
keywordHandler:addAliasKeyword({'shield'})
keywordHandler:addKeyword({'embrace'}, StdModule.say, {npcHandler = npcHandler, text = 'The druids north of Carlin will provide you with the embrace of Tibia.'})
keywordHandler:addKeyword({'phoenix'}, StdModule.say, {npcHandler = npcHandler, text = 'The spark of the phoenix is given by the dwarven priests of earth and fire in Kazordoon.'})
keywordHandler:addAliasKeyword({'spark'})
keywordHandler:addKeyword({'solitude'}, StdModule.say, {npcHandler = npcHandler, text = 'Talk to the hermit Eremo on the isle of Cormaya about this blessing.'})
keywordHandler:addAliasKeyword({'wisdom'})

npcHandler:setMessage(MESSAGE_WALKAWAY, 'Asha Thrazi, |PLAYERNAME|!')
npcHandler:setMessage(MESSAGE_FAREWELL, 'Asha Thrazi, |PLAYERNAME|!')

npcHandler:setCallback(CALLBACK_GREET, greetCallback)

local focusModule = FocusModule:new()
focusModule:addGreetMessage({'hi', 'hello', 'ashari'})
focusModule:addFarewellMessage({'bye', 'farewell', 'asgha thrazi'})
npcHandler:addModule(focusModule)


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

npcType:register()
