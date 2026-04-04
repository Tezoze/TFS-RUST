-- Maealil - Converted from XML to Lua NpcType
-- Original XML: data/npc/Maealil.xml
-- Original Script: data/npc/scripts/Maealil.lua

local npcName = "Maealil"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a maealil")
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



local function creatureSayCallback(cid, type, msg)
	if not npcHandler:isFocused(cid) then
		return false
	end
	
	local player = Player(cid)
	if not player then
		return false
	end
	
	local pvpBlessCost = StdModule.calculateRegularBlessingCost(player:getLevel())
	
	-- Twist of Fate
	if msgcontains(msg, "twist") or msgcontains(msg, "fate") then
		npcHandler:say({
			'This is a special blessing I can bestow upon you once you have obtained at least one of the other blessings and which functions a bit differently. ...',
			'It only works when you\'re killed by other adventurers, which means that at least half of the damage leading to your death was caused by others, not by monsters or the environment. ...',
			'The {twist of fate} will not reduce the death penalty like the other blessings, but instead prevent you from losing your other blessings as well as the amulet of loss, should you wear one. It costs the same as the other blessings. ...',
			'Would you like to receive that protection for a sacrifice of ' .. pvpBlessCost .. ' gold, child?'
		}, cid)
		npcHandler.topic[cid] = 1
		return true
	elseif msgcontains(msg, "yes") then
		if npcHandler.topic[cid] == 1 then
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
		if npcHandler.topic[cid] > 0 then
			npcHandler:say('Fine. You are free to decline my offer.', cid)
			npcHandler.topic[cid] = 0
			return true
		end
	end

	-- Adventurer Stone
	if msgcontains(msg, "adventurer") and msgcontains(msg, "stone") then
		if player:getItemById(18559, true) then
			npcHandler:say('Keep your adventurer\'s stone well.', cid)
		elseif player:getStorageValue(Storage.AdventurersGuild.FreeStone.Maealil) ~= 1 then
			npcHandler:say('Ah, you want to replace your adventurer\'s stone for free?', cid)
			npcHandler.topic[cid] = 2
		else
			npcHandler:say('Ah, you want to replace your adventurer\'s stone for 30 gold?', cid)
			npcHandler.topic[cid] = 3
		end
		return true

	-- Wooden Stake Quest
	elseif msgcontains(msg, "stake") then
		local stakeStorage = player:getStorageValue(Storage.FriendsAndTraders.TheBlessedStake)

		if stakeStorage == 3 and player:getItemCount(5941) == 0 then
			npcHandler:say('I think you have forgotten to bring your stake.', cid)
		elseif stakeStorage == 3 then
			npcHandler:say('Yes, I was informed what to do. Are you prepared to receive my line of the prayer?', cid)
			npcHandler.topic[cid] = 4
		elseif stakeStorage == 4 then
			npcHandler:say('You should visit Yberius in the Venore temple now.', cid)
		elseif stakeStorage > 4 then
			npcHandler:say('You already received my line of the prayer.', cid)
		else
			npcHandler:say('A blessed stake? That is a strange request. Maybe Quentin knows more, he is one of the oldest monks after all.', cid)
		end
		return true

	-- Healing
	elseif msgcontains(msg, "heal") then
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

	-- Basic keywords
	elseif msgcontains(msg, "pilgrimage") then
		npcHandler:say('Whenever you receive a lethal wound, your vital force is damaged and there is a chance that you lose some of your equipment. With every single of the five {blessings} you have, this damage and chance of loss will be reduced.', cid)
		return true
	elseif msgcontains(msg, "blessings") then
		npcHandler:say('There are five blessings available in five sacred places: the {spiritual} shielding, the spark of the {phoenix}, the {embrace} of Tibia, the fire of the {suns} and the wisdom of {solitude}. Additionally, you can receive the {twist of fate} here.', cid)
		return true
	elseif msgcontains(msg, "spiritual") or msgcontains(msg, "shield") then
		if player:hasBlessing(1) then
			npcHandler:say('I see you received the spiritual shielding in the whiteflower temple south of Thais.', cid)
		else
			npcHandler:say('You can ask for the blessing of spiritual shielding in the whiteflower temple south of Thais.', cid)
		end
		return true
	elseif msgcontains(msg, "embrace") then
		if player:hasBlessing(2) then
			npcHandler:say('I can sense that the druids north of Carlin have provided you with the Embrace of Tibia.', cid)
		else
			npcHandler:say('The druids north of Carlin will provide you with the embrace of Tibia.', cid)
		end
		return true
	elseif msgcontains(msg, "suns") or msgcontains(msg, "fire") then
		if player:hasBlessing(3) then
			npcHandler:say('I can see you received the blessing of the two suns in the suntower near Ab\'Dendriel.', cid)
		else
			npcHandler:say('You can ask for the blessing of the two suns in the suntower near Ab\'Dendriel.', cid)
		end
		return true
	elseif msgcontains(msg, "phoenix") or msgcontains(msg, "spark") then
		if player:hasBlessing(4) then
			npcHandler:say('I can sense that the spark of the phoenix already was given to you by the dwarven priests of earth and fire in Kazordoon.', cid)
		else
			npcHandler:say('The spark of the phoenix is given by the dwarven priests of earth and fire in Kazordoon.', cid)
		end
		return true
	elseif msgcontains(msg, "solitude") or msgcontains(msg, "wisdom") then
		if player:hasBlessing(5) then
			npcHandler:say('I can sense you already talked to the hermit Eremo on the isle of Cormaya and received this blessing.', cid)
		else
			npcHandler:say('Talk to the hermit Eremo on the isle of Cormaya about this blessing.', cid)
		end
		return true
	end

	-- Handle yes/no responses for adventurer stone and stake quest
	if npcHandler.topic[cid] == 2 and msgcontains(msg, "yes") then
		npcHandler:say('Here you are. Take care.', cid)
		player:addItem(18559, 1)
		player:setStorageValue(Storage.AdventurersGuild.FreeStone.Maealil, 1)
		npcHandler.topic[cid] = 0
		return true
	elseif npcHandler.topic[cid] == 2 and msgcontains(msg, "no") then
		npcHandler:say('No problem.', cid)
		npcHandler.topic[cid] = 0
		return true
	elseif npcHandler.topic[cid] == 3 and msgcontains(msg, "yes") then
		if player:getMoney() + player:getBankBalance() >= 30 then
			if player:removeMoneyNpc(30) then
				npcHandler:say('Here you are. Take care.', cid)
				player:addItem(18559, 1)
			end
		else
			npcHandler:say('Sorry, you don\'t have enough money.', cid)
		end
		npcHandler.topic[cid] = 0
		return true
	elseif npcHandler.topic[cid] == 3 and msgcontains(msg, "no") then
		npcHandler:say('No problem.', cid)
		npcHandler.topic[cid] = 0
		return true
	elseif npcHandler.topic[cid] == 4 and msgcontains(msg, "yes") then
		npcHandler:say('So receive my prayer: \'Peace may fill your soul - evil shall be cleansed\'. Now, bring your stake to Yberius in the Venore temple for the next line of the prayer. I will inform him what to do.', cid)
		player:setStorageValue(Storage.FriendsAndTraders.TheBlessedStake, 4)
		player:getPosition():sendMagicEffect(CONST_ME_MAGIC_BLUE)
		npcHandler.topic[cid] = 0
		return true
	elseif npcHandler.topic[cid] == 4 and msgcontains(msg, "no") then
		npcHandler:say('I will wait for you.', cid)
		npcHandler.topic[cid] = 0
		return true
	end

	return false
end

npcHandler:setCallback(CALLBACK_MESSAGE_DEFAULT, creatureSayCallback)

npcHandler:setMessage(MESSAGE_GREET, 'Welcome, young |PLAYERNAME|! If you are heavily wounded or poisoned, I can {heal} you for free.')
npcHandler:setMessage(MESSAGE_WALKAWAY, 'Remember: If you are heavily wounded or poisoned, I can heal you for free.')
npcHandler:setMessage(MESSAGE_FAREWELL, 'May the gods bless you, |PLAYERNAME|!')

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
