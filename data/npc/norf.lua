-- Norf - Converted from XML to Lua NpcType
-- Original XML: data/npc/Norf.xml
-- Original Script: data/npc/scripts/Norf.lua

local npcName = "Norf"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a norf")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(2000)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 57})
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

-- Spiritual Shielding
local blessKeyword = keywordHandler:addKeyword({'spiritual'}, function(cid, message, keywords, parameters, node)
	local npcHandler = parameters.npcHandler
	if not npcHandler:isFocused(cid) then
		return false
	end
	
	local player = Player(cid)
	if not player then
		return false
	end
	
	local blessCost = StdModule.calculateRegularBlessingCost(player:getLevel())
	npcHandler:say('Here in the whiteflower temple you may receive the blessing of spiritual shielding. But we must ask of you to sacrifice ' .. blessCost .. ' gold. Are you still interested?', cid)
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
		
		if player:hasBlessing(1) then
			npcHandler:say("Gods have already blessed you with this blessing!", cid)
		elseif not player:removeTotalMoney(blessCost) then
			npcHandler:say("You don't have enough money for blessing.", cid)
		else
			player:addBlessing(1)
			npcHandler:say("So receive the shielding of your spirit, pilgrim.", cid)
			player:getPosition():sendMagicEffect(CONST_ME_MAGIC_BLUE)
		end
		npcHandler.topic[cid] = 0
		npcHandler:resetNpc(cid)
		return true
	end, {npcHandler = npcHandler})
	blessKeyword:addChildKeyword({''}, StdModule.say, {npcHandler = npcHandler, text = 'Fine. You are free to decline my offer.', reset = true})
keywordHandler:addAliasKeyword({'shield'})

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
	if player:getStorageValue(Storage.PilgrimageOfAshes.Questline) ~= 1 then
		npcHandler:say("I sense you are not on the pilgrimage of ashes. You should start this quest with one of the city guides first.", cid)
		return true
	end

	-- Check if player has already completed this blessing
	if player:getStorageValue(Storage.PilgrimageOfAshes.Mission01) >= 2 then
		npcHandler:say("You have already received the blessing of spiritual shielding. You should continue your pilgrimage to the next sacred place.", cid)
		return true
	end


	local blessCost = StdModule.calculateRegularBlessingCost(player:getLevel()) - 1000
	npcHandler:say('So you were sent on the pilgrimage of ashes... I see. Here in the whiteflower temple you may receive the blessing of spiritual shielding. But I still must ask of you to sacrifice ' .. blessCost .. ' gold. Are you still interested?', cid)
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

		if player:hasBlessing(1) then
			npcHandler:say("Gods have already blessed you with this blessing!", cid)
		elseif not player:removeTotalMoney(blessCost) then
			npcHandler:say("You don't have enough money for blessing.", cid)
		else
			player:addBlessing(1)
			player:setStorageValue(Storage.PilgrimageOfAshes.Mission01, 2)
			player:setStorageValue(Storage.PilgrimageOfAshes.Questline, 2)
			player:setStorageValue(Storage.PilgrimageOfAshes.Mission02, 1)
			player:addMapMark(Position(32359, 31687, 7), MAPMARK_GREENNORTH, "Stone Circle")
			npcHandler:say({
				"So receive the shielding of your spirit, pilgrim. This is the first of five available blessings. Let me tell you where you will find the second sacred place: ...",
				"North of Carlin lies a great stone circle. To the right of that, the druid Humphrey will provide you with the embrace of Tibia. I also mark the spot on your map. Good luck on your pilgrimage."
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
keywordHandler:addKeyword({'embrace'}, StdModule.say, {npcHandler = npcHandler, text = 'I can sense that the druids north of Carlin have provided you with the Embrace of Tibia.'}, function(player) return player:hasBlessing(2) end)
keywordHandler:addKeyword({'suns'}, StdModule.say, {npcHandler = npcHandler, text = 'I can see you received the blessing of the two suns in the suntower near Ab\'Dendriel.'}, function(player) return player:hasBlessing(3) end)
keywordHandler:addAliasKeyword({'fire'})
keywordHandler:addKeyword({'phoenix'}, StdModule.say, {npcHandler = npcHandler, text = 'I can sense that the spark of the phoenix already was given to you by the dwarven priests of earth and fire in Kazordoon.'}, function(player) return player:hasBlessing(4) end)
keywordHandler:addAliasKeyword({'spark'})
keywordHandler:addKeyword({'solitude'}, StdModule.say, {npcHandler = npcHandler, text = 'I can sense you already talked to the hermit Eremo on the isle of Cormaya and received this blessing.'}, function(player) return player:hasBlessing(5) end)
keywordHandler:addAliasKeyword({'wisdom'})
keywordHandler:addKeyword({'embrace'}, StdModule.say, {npcHandler = npcHandler, text = 'The druids north of Carlin will provide you with the embrace of Tibia.'})
keywordHandler:addKeyword({'suns'}, StdModule.say, {npcHandler = npcHandler, text = 'You can ask for the blessing of the two suns in the suntower near Ab\'Dendriel.'})
keywordHandler:addAliasKeyword({'fire'})
keywordHandler:addKeyword({'phoenix'}, StdModule.say, {npcHandler = npcHandler, text = 'The spark of the phoenix is given by the dwarven priests of earth and fire in Kazordoon.'})
keywordHandler:addAliasKeyword({'spark'})
keywordHandler:addKeyword({'solitude'}, StdModule.say, {npcHandler = npcHandler, text = 'Talk to the hermit Eremo on the isle of Cormaya about this blessing.'})
keywordHandler:addAliasKeyword({'wisdom'})

npcHandler:setMessage(MESSAGE_GREET, 'Welcome, pilgrim. How may I {help} you? Are you in need of {healing}?')


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
