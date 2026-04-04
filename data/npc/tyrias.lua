-- Tyrias - Converted from XML to Lua NpcType
-- Original XML: data/npc/Tyrias.xml
-- Original Script: data/npc/scripts/Tyrias.lua

local npcName = "Tyrias"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a tyrias")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(2000)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 133, lookHead = 57, lookBody = 113, lookLegs = 95, lookFeet = 113})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)

-- NOTE: This function is kept for compatibility but the main logic is now in creatureSayCallback

-- Main callback
local function creatureSayCallback(cid, type, msg)
	if not npcHandler:isFocused(cid) then
		return false
	end

	local player = Player(cid)
	if not player then
		return false
	end

	local pvpBlessCost = StdModule.calculateRegularBlessingCost(player:getLevel())

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
			if health < 40 then player:addHealth(40 - health) end
			player:getPosition():sendMagicEffect(CONST_ME_MAGIC_GREEN)
			healed = true
		end

		if not healed then
			npcHandler:say("You aren't looking that bad. Sorry, I can't help you. But if you are looking for additional protection you should go on the {pilgrimage} of ashes or get the protection of the {twist of fate} here.", cid)
		end

		return true

	-- Wooden Stake
	elseif msgcontains(msg, "stake") then
		local stakeStorage = player:getStorageValue(Storage.FriendsAndTraders.TheBlessedStake)

		if stakeStorage == 10 and player:getItemCount(5941) == 0 then
			npcHandler:say('You don\'t even have that strange stake with you.', cid)
		elseif stakeStorage == 10 then
			npcHandler:say('Brewster sent me a strange message about some strange hocus-pocus. I think it\'s nonsense, but since you have come that far, I\'ll play along. Are you ready?', cid)
			npcHandler.topic[cid] = 10
		elseif stakeStorage == 11 then
			npcHandler:say('I won\'t do that rubbish again. Go pester Chondur with your hocus-pocus.', cid)
		else
			npcHandler:say('A blessed stake? I don\'t believe in things like that. If anyone does, it\'s probably old Quentin.', cid)
		end
		return true

	-- Twist of Fate
	elseif msgcontains(msg, "twist") or msgcontains(msg, "fate") then
		npcHandler:say({
			'This is a special blessing I can bestow upon you once you have obtained at least one of the other blessings and which functions a bit differently. ...',
			'It only works when you\'re killed by other adventurers, which means that at least half of the damage leading to your death was caused by others, not by monsters or the environment. ...',
			'The {twist of fate} will not reduce the death penalty like the other blessings, but instead prevent you from losing your other blessings as well as the amulet of loss, should you wear one. It costs the same as the other blessings. ...',
			'Would you like to receive that protection for a sacrifice of ' .. pvpBlessCost .. ' gold, child?'
		}, cid)
		npcHandler.topic[cid] = 1
		return true
	elseif msgcontains(msg, "yes") then
		if npcHandler.topic[cid] == 10 then
			if player:getItemCount(5941) > 0 then
				npcHandler:say('So receive my prayer: \'Your mind shall be a vessel for joy, light and wisdom\' - uh, wow, something happened. Well, I guess that\'s it, but next time if you need some mumbo jumbo rather go to Chondur.', cid)
				player:setStorageValue(Storage.FriendsAndTraders.TheBlessedStake, 11)
				player:removeItem(5941, 1)
				player:addItem(5942, 1)
				player:getPosition():sendMagicEffect(CONST_ME_MAGIC_BLUE)
			else
				npcHandler:say('You don\'t even have that strange stake with you.', cid)
			end
			npcHandler.topic[cid] = 0
			return true
		elseif npcHandler.topic[cid] == 1 then
			if player:hasBlessing(6) then
				npcHandler:say("Gods have already blessed you with this blessing!", cid)
			elseif not player:removeTotalMoney(pvpBlessCost) then
				npcHandler:say("You don't have enough money for blessing.", cid)
			else
				player:addBlessing(6)
				npcHandler:say("So receive the protection of the twist of fate, pilgrim.", cid)
				player:getPosition():sendMagicEffect(CONST_ME_MAGIC_BLUE)
			end
			npcHandler.topic[cid] = 0
			return true
		end
	elseif msgcontains(msg, "no") then
		if npcHandler.topic[cid] == 10 then
			npcHandler:say('No problem, I have other things to do.', cid)
			npcHandler.topic[cid] = 0
			return true
		elseif npcHandler.topic[cid] > 0 then
			npcHandler:say('Fine. You are free to decline my offer.', cid)
			npcHandler.topic[cid] = 0
			return true
		end
	end

	-- Return false to let keyword handlers process other keywords
	return false
end

npcHandler:setCallback(CALLBACK_MESSAGE_DEFAULT, creatureSayCallback)

-- Adventurer Stone
keywordHandler:addKeyword({'adventurer stone'}, StdModule.say, {npcHandler = npcHandler, text = 'Keep your adventurer\'s stone well.'}, function(player) return player:getItemById(18559, true) end)

local stoneKeyword = keywordHandler:addKeyword({'adventurer stone'}, StdModule.say, {npcHandler = npcHandler, text = 'Ah, you want to replace your adventurer\'s stone for free?'}, function(player) return player:getStorageValue(Storage.AdventurersGuild.FreeStone.Tyrias) ~= 1 end)
	stoneKeyword:addChildKeyword({'yes'}, StdModule.say, {npcHandler = npcHandler, text = 'Here you are. Take care.', reset = true}, nil, function(player) player:addItem(18559, 1) player:setStorageValue(Storage.AdventurersGuild.FreeStone.Tyrias, 1) end)
	stoneKeyword:addChildKeyword({''}, StdModule.say, {npcHandler = npcHandler, text = 'No problem.', reset = true})

local stoneKeyword = keywordHandler:addKeyword({'adventurer stone'}, StdModule.say, {npcHandler = npcHandler, text = 'Ah, you want to replace your adventurer\'s stone for 30 gold?'})
	stoneKeyword:addChildKeyword({'yes'}, StdModule.say, {npcHandler = npcHandler, text = 'Here you are. Take care.', reset = true},
		function(player) return player:getMoney() + player:getBankBalance() >= 30 end,
		function(player) if player:removeMoneyNpc(30) then player:addItem(18559, 1) end end
	)
	stoneKeyword:addChildKeyword({'yes'}, StdModule.say, {npcHandler = npcHandler, text = 'Sorry, you don\'t have enough money.', reset = true})
	stoneKeyword:addChildKeyword({''}, StdModule.say, {npcHandler = npcHandler, text = 'No problem.', reset = true})

-- Basic blessing keywords are now handled in the callback

npcHandler:setMessage(MESSAGE_GREET, 'Welcome, young |PLAYERNAME|! If you are heavily wounded or poisoned, I can {heal} you for free.')
npcHandler:setMessage(MESSAGE_WALKAWAY, 'Remember: If you are heavily wounded or suffering from conditions, I can heal you for free.')
npcHandler:setMessage(MESSAGE_FAREWELL, 'May the gods bless you, |PLAYERNAME|!')


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
