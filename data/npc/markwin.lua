-- Markwin - Converted from XML to Lua NpcType
-- Original XML: data/npc/Markwin.xml
-- Original Script: data/npc/scripts/Markwin.lua

local npcName = "Markwin"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a markwin")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(2000)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 23})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)


local condition = Condition(CONDITION_FIRE)
condition:setParameter(CONDITION_PARAM_TICKS, 30 * 1000)
condition:setParameter(CONDITION_PARAM_MINVALUE, 30)
condition:setParameter(CONDITION_PARAM_TICKINTERVAL, 4000)

local guards = { "Minotaur Guard", "Minotaur Archer", "Minotaur Mage" }
local function greetCallback(cid)
	local player = Player(cid)
	if player:getStorageValue(Storage.MarkwinGreeting) < 1 then
		npcHandler:setMessage(MESSAGE_GREET, "Intruder! Guards, take him down!")
		player:setStorageValue(Storage.MarkwinGreeting, 1)
		local position
		for x = -1, 1 do
			for y = -1, 1 do
				position = Position(32418 + x, 32147 + y, 15)
				Game.createMonster(guards[math.random(3)], position)
				position:sendMagicEffect(CONST_ME_TELEPORT)
			end
		end
		return false
	elseif player:getStorageValue(Storage.MarkwinGreeting) == 1 then
		npcHandler:setMessage(MESSAGE_GREET, "Well ... you defeated my guards! Now everything is over! I guess I will have to answer your questions now.")
		player:setStorageValue(Storage.MarkwinGreeting, 2)
	elseif player:getStorageValue(Storage.MarkwinGreeting) == 2 then
		npcHandler:setMessage(MESSAGE_GREET, "Oh its you again. What du you want, hornless messenger?")
	end
	return true
end

local function creatureSayCallback(cid, type, msg)
	if not npcHandler:isFocused(cid) then
		return false
	end

	local player = Player(cid)
	if msgcontains(msg, "letter") then
		if player:getStorageValue(Storage.postman.Mission10) == 1 then
			if player:getItemCount(2333) > 0 then
				npcHandler:say("A letter from my Moohmy?? Do you have a letter from my Moohmy to me?", cid)
				npcHandler.topic[cid] = 1
			end
		end
	elseif msgcontains(msg, 'cookie') then
		if player:getStorageValue(Storage.WhatAFoolishQuest.Questline) == 31
				and player:getStorageValue(Storage.WhatAFoolishQuest.CookieDelivery.Markwin) ~= 1 then
			npcHandler:say('You bring me ... a cookie???', cid)
			npcHandler.topic[cid] = 2
		end
	elseif msgcontains(msg, "yes") then
		if npcHandler.topic[cid] == 1 then
			npcHandler:say("Uhm, well thank you, hornless being.", cid)
			player:setStorageValue(Storage.postman.Mission10, 2)
			player:removeItem(2333, 1)
			npcHandler.topic[cid] = 0
		elseif npcHandler.topic[cid] == 2 then
			if not player:removeItem(8111, 1) then
				npcHandler:say('You have no cookie that I\'d like.', cid)
				npcHandler.topic[cid] = 0
				return true
			end

			player:setStorageValue(Storage.WhatAFoolishQuest.CookieDelivery.Markwin, 1)
			if player:getCookiesDelivered() == 10 then
				player:addAchievement('Allow Cookies?')
			end

			Npc():getPosition():sendMagicEffect(CONST_ME_GIFT_WRAPS)
			npcHandler:say('I understand this as a peace-offering, human ... UNGH ... THIS IS AN OUTRAGE! THIS MEANS WAR!!!', cid)
			npcHandler:releaseFocus(cid)
			npcHandler:resetNpc(cid)
		end
	elseif msgcontains(msg, "bye") then
		npcHandler:say("Hm ... good bye.", cid)
		player:addCondition(condition)
		npcHandler:releaseFocus(cid)
		npcHandler:resetNpc(cid)
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
