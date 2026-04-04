-- Wyda - Converted from XML to Lua NpcType
-- Original XML: data/npc/Wyda.xml
-- Original Script: data/npc/scripts/Wyda.lua

local npcName = "Wyda"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a wyda")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(2000)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 54, lookBody = 119, lookLegs = 119, lookFeet = 126})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)


local condition = Condition(CONDITION_FIRE)
condition:setParameter(CONDITION_PARAM_DELAYED, 1)
condition:addDamage(60, 2000, -10)

local function creatureSayCallback(cid, type, msg)
	if not npcHandler:isFocused(cid) then
		return false
	end

	local player = Player(cid)

	if msgcontains(msg, 'cookie') then
		if player:getStorageValue(Storage.WhatAFoolishQuest.Questline) == 31
				and player:getStorageValue(Storage.WhatAFoolishQuest.CookieDelivery.Wyda) ~= 1 then
			npcHandler:say('You brought me a cookie?', cid)
			npcHandler.topic[cid] = 1
		end
	elseif msgcontains(msg, 'mission') or msgcontains(msg, 'quest') then
		npcHandler:say({
			"A quest? Well, if you\'re so keen on doing me a favour... Why don\'t you try to find a {blood herb}?",
			"To be honest, I\'m drowning in blood herbs by now."
		}, cid)
		npcHandler.topic[cid] = 0
	elseif msgcontains(msg, 'bloodherb') or msgcontains(msg, 'blood herb') then
		if player:getStorageValue(Storage.BloodHerbQuest) == 1  then
			npcHandler:say('Arrr... here we go again.... do you have a #$*§# blood herb for me?', cid)
			npcHandler.topic[cid] = 2
		else
			npcHandler:say({
				"The blood herb is very rare. This plant would be very useful for me, but I don't know any accessible places to find it.",
				"To be honest, I'm drowning in blood herbs by now. But if it helps you, well yes.. I guess I could use another blood herb..."
			}, cid)
			npcHandler.topic[cid] = 0
		end
	elseif msgcontains(msg, 'yes') then
		if npcHandler.topic[cid] == 1 then
			if not player:removeItem(8111, 1) then
				npcHandler:say('You have no cookie that I\'d like.', cid)
				npcHandler.topic[cid] = 0
				return true
			end

			player:setStorageValue(Storage.WhatAFoolishQuest.CookieDelivery.Wyda, 1)
			player:addCondition(condition)
			if player:getCookiesDelivered() == 10 then
				player:addAchievement('Allow Cookies?')
			end

			Npc():getPosition():sendMagicEffect(CONST_ME_GIFT_WRAPS)
			npcHandler:say('Well, it\'s a welcome change from all that gingerbread ... AHHH HOW DARE YOU??? FEEL MY WRATH!', cid)
			npcHandler:releaseFocus(cid)
			npcHandler:resetNpc(cid)
		elseif npcHandler.topic[cid] == 2 then
			if player:removeItem(2798, 1) then
				player:setStorageValue(Storage.BloodHerbQuest, 2)
				player:getPosition():sendMagicEffect(CONST_ME_MAGIC_GREEN)
				local TornTeddyRand = math.random(1, 100)
				if TornTeddyRand <= 70 then
					player:addItem(2324, 1) -- witchesbroom
					npcHandler:say('Thank you -SOOO- much! No, I really mean it! Really! Here, let me give you a reward...', cid)
					npcHandler.topic[cid] = 0
				else
					player:addItem(13774, 1) -- torn teddy
					npcHandler:say('Thank you -SOOO- much! No, I really mean it! Really! Ah, you know what, you can have this old thing...', cid)
					npcHandler.topic[cid] = 0
				end
			else
				npcHandler:say('No, you don\'t have any...', cid)
				npcHandler.topic[cid] = 0
			end
		end
	elseif msgcontains(msg, 'no') then
		if npcHandler.topic[cid] == 1 or npcHandler.topic[cid] == 2 then
			npcHandler:say('I see.', cid)
			npcHandler.topic[cid] = 0
		end
	end
	return true
end

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
