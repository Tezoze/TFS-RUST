-- Tibra - Converted from XML to Lua NpcType
-- Original XML: data/npc/Tibra.xml
-- Original Script: data/npc/scripts/Tibra.lua

local npcName = "Tibra"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a tibra")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(2000)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 138, lookHead = 41, lookBody = 92, lookLegs = 90, lookFeet = 95})
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

	-- Confession
	if msgcontains(msg, "sins") then
		npcHandler:say('Do you wish to confess your sins, my child?', cid)
		npcHandler.topic[cid] = 1
		return true
	-- Donation
	elseif msgcontains(msg, "donation") then
		npcHandler:say('Do you want to make a donation?', cid)
		npcHandler.topic[cid] = 4
		return true
	-- Twist of Fate
	elseif msgcontains(msg, "twist") or msgcontains(msg, "fate") then
		npcHandler:say({
			'This is a special blessing I can bestow upon you once you have obtained at least one of the other blessings and which functions a bit differently. ...',
			'It only works when you\'re killed by other adventurers, which means that at least forty percent of the damage leading to your death was caused by others, not by monsters or the environment. ...',
			'The twist of fate will not reduce the death penalty like the other blessings, but instead prevent you from losing your other blessings as well as the amulet of loss, should you wear one. It costs the same as the other blessings. ...',
			'Would you like to receive that protection for a sacrifice of ' .. pvpBlessCost .. ' gold, child?'
		}, cid)
		npcHandler.topic[cid] = 2
		return true
	elseif msgcontains(msg, "yes") then
		if npcHandler.topic[cid] == 1 then
			npcHandler:say('So tell me, what shadows your soul, my child.', cid)
			npcHandler.topic[cid] = 5
			return true
		elseif npcHandler.topic[cid] == 4 then
			npcHandler:say('May the gods bless you!', cid)
			npcHandler.topic[cid] = 0
			return true
		elseif npcHandler.topic[cid] == 2 then
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
		if npcHandler.topic[cid] == 2 then
			npcHandler:say('Fine. You are free to decline my offer.', cid)
			npcHandler.topic[cid] = 0
			return true
		end
	end

	-- Wooden Stake Quest
	if msgcontains(msg, "stake") then
		local stakeStorage = player:getStorageValue(Storage.FriendsAndTraders.TheBlessedStake)

		if stakeStorage == 2 and player:getItemCount(5941) == 0 then
			npcHandler:say('I think you have forgotten to bring your stake, my child.', cid)
		elseif stakeStorage == 2 then
			npcHandler:say('Yes, I was informed what to do. Are you prepared to receive my line of the prayer?', cid)
			npcHandler.topic[cid] = 3
		elseif stakeStorage == 3 then
			npcHandler:say('You should visit Maealil in the elven settlement now, my child.', cid)
		elseif stakeStorage > 3 then
			npcHandler:say('You already received my line of the prayer, dear child.', cid)
		else
			npcHandler:say('A blessed stake? That is a strange request, my child. Maybe Quentin knows more, he is one of the oldest monks after all.', cid)
		end
		return true
	elseif msgcontains(msg, "yes") then
		if npcHandler.topic[cid] == 3 then
			npcHandler:say('So receive my prayer: \'Hope may fill your heart - doubt shall be banned\'. Now, bring your stake to Maealil in the elven settlement for the next line of the prayer. I will inform him what to do.', cid)
			player:setStorageValue(Storage.FriendsAndTraders.TheBlessedStake, 3)
			player:getPosition():sendMagicEffect(CONST_ME_MAGIC_BLUE)
			npcHandler.topic[cid] = 0
			return true
		end
	elseif msgcontains(msg, "no") then
		if npcHandler.topic[cid] == 3 then
			npcHandler:say('I will wait for you.', cid)
			npcHandler.topic[cid] = 0
			return true
		end
		if npcHandler.topic[cid] == 2 then
			npcHandler:say('Fine. You are free to decline my offer.', cid)
			npcHandler.topic[cid] = 0
			return true
		end
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
			npcHandler:say("You aren't looking that bad. Sorry, I need my powers for cases more severe than yours.", cid)
		end

		return true
	end

	-- Confession response
	if npcHandler.topic[cid] == 5 then
		npcHandler:say('Meditate on that and pray for your soul.', cid)
		npcHandler.topic[cid] = 0
		return true
	-- Basic keywords
	elseif msgcontains(msg, "pilgrimage") then
		npcHandler:say('Whenever you receive a lethal wound, your vital force is damaged and there is a chance that you lose some of your equipment. With every single of the five {blessings} you have, this damage and chance of loss will be reduced.', cid)
		return true
	elseif msgcontains(msg, "blessings") then
		npcHandler:say('There are five blessings available in five sacred places: the {spiritual} shielding, the spark of the {phoenix}, the {embrace} of Tibia, the fire of the {suns} and the wisdom of {solitude}. Additionally, you can receive the {twist of fate} here.', cid)
		return true
	elseif msgcontains(msg, "spiritual") then
		npcHandler:say('I see you received the spiritual shielding in the whiteflower temple south of Thais.', cid)
		return true
	elseif msgcontains(msg, "phoenix") then
		npcHandler:say('I can sense that the spark of the phoenix already was given to you by the dwarven priests of earth and fire in Kazordoon.', cid)
		return true
	elseif msgcontains(msg, "embrace") then
		npcHandler:say('I can sense that the druids north of Carlin have provided you with the Embrace of Tibia.', cid)
		return true
	elseif msgcontains(msg, "suns") then
		npcHandler:say('I can see you received the blessing of the two suns in the suntower near Ab\'Dendriel.', cid)
		return true
	elseif msgcontains(msg, "solitude") then
		npcHandler:say('I can sense you already talked to the hermit Eremo on the isle of Cormaya and received this blessing.', cid)
		return true
	elseif msgcontains(msg, "job") then
		npcHandler:say('I am a priest of the great pantheon.', cid)
		return true
	elseif msgcontains(msg, "life") then
		npcHandler:say('The teachings of Crunor tell us to honor life and not to harm it.', cid)
		return true
	elseif msgcontains(msg, "mission") or msgcontains(msg, "quest") then
		npcHandler:say('It is my mission to bring the teachings of the gods to everyone.', cid)
		return true
	elseif msgcontains(msg, "name") then
		npcHandler:say('My name is Tibra. Your soul tells me that you are |PLAYERNAME|.', cid)
		return true
	elseif msgcontains(msg, "queen") then
		npcHandler:say('Queen Eloise is wise to listen to the proposals of the druidic followers of Crunor.', cid)
		return true
	elseif msgcontains(msg, "sell") then
		npcHandler:say('The grace of the gods must be earned, it cannot be bought!', cid)
		return true
	elseif msgcontains(msg, "tibia") then
		npcHandler:say('The world of Tibia is the creation of the gods.', cid)
		return true
	elseif msgcontains(msg, "time") then
		npcHandler:say('Now, it is 11:51 pm.', cid)
		return true
	elseif msgcontains(msg, "crypt") then
		npcHandler:say('There\'s something strange in its neighbourhood. But whom we gonna call for help if not the gods?', cid)
		return true
	elseif msgcontains(msg, "help") then
		npcHandler:say('You aren\'t looking that bad. Sorry, I need my powers for cases more severe than yours.', cid)
		return true
	elseif msgcontains(msg, "monsters") then
		npcHandler:say('Remind: Not everything you call monster is evil to the core!', cid)
		return true
	elseif msgcontains(msg, "excalibug") then
		npcHandler:say('The mythical blade was hidden in ancient times. Its said that powerful wards protect it.', cid)
		return true
	elseif msgcontains(msg, "ferumbras") then
		npcHandler:say('The fallen one should be mourned, not feared.', cid)
		return true
	elseif msgcontains(msg, "lugri") then
		npcHandler:say('Only a man can fall as low as he did. His soul rotted away already. Sins', cid)
		return true
	elseif msgcontains(msg, "gods") and not msgcontains(msg, "good") and not msgcontains(msg, "evil") then
		npcHandler:say('The gods of good guard us and guide us, the gods of evil want to destroy us and steal our souls!', cid)
		return true
	elseif msgcontains(msg, "gods of good") then
		npcHandler:say('The gods we call the good ones are Fardos, Uman, the Elements, Suon, Crunor, Nornur, Bastesh, Kirok, Toth, and Banor.', cid)
		return true
	elseif msgcontains(msg, "fardos") then
		npcHandler:say('Fardos is the creator. The great obsever. He is our caretaker.', cid)
		return true
	elseif msgcontains(msg, "uman") then
		npcHandler:say('Uman is the positive aspect of magic. He brings us the secrets of the arcane arts.', cid)
		return true
	elseif msgcontains(msg, "air") then
		npcHandler:say('Air is one of the primal elemental forces, sometimes worshipped by tribal shamans.', cid)
		return true
	elseif msgcontains(msg, "fire") then
		npcHandler:say('Fire is one of the primal elemental forces, sometimes worshipped by tribal shamans.', cid)
		return true
	elseif msgcontains(msg, "sula") then
		npcHandler:say('Sula is the essence of the elemental power of water.', cid)
		return true
	elseif msgcontains(msg, "suon") then
		npcHandler:say('Suon is the lifebringing sun. He observes the creation with love.', cid)
		return true
	elseif msgcontains(msg, "crunor") then
		npcHandler:say('Crunor, the great tree, is the father of all plantlife. He is a prominent god for many druids.', cid)
		return true
	elseif msgcontains(msg, "nornur") then
		npcHandler:say('Nornur is the mysterious god of fate. Who knows if he is its creator or just a chronist?', cid)
		return true
	elseif msgcontains(msg, "bastesh") then
		npcHandler:say('Bastesh, the deep one, is the goddess of the sea and its creatures.', cid)
		return true
	elseif msgcontains(msg, "kirok") then
		npcHandler:say('Kirok, the mad one, is the god of scientists and jesters.', cid)
		return true
	elseif msgcontains(msg, "toth") then
		npcHandler:say('Toth, Lord of Death, is the keeper of the souls, the guardian of the afterlife.', cid)
		return true
	elseif msgcontains(msg, "banor") then
		npcHandler:say('Banor, the heavenly warrior, is the patron of all fighters against evil. He is the gift of the gods to inspire humanity.', cid)
		return true
	elseif msgcontains(msg, "evil") then
		npcHandler:say('The gods we call the evil ones are Zathroth, Fafnar, Brog, Urgith, and the Archdemons!', cid)
		return true
	elseif msgcontains(msg, "zathroth") then
		npcHandler:say('Zathroth is the destructive aspect of magic. He is the deciver and the thief of souls.', cid)
		return true
	elseif msgcontains(msg, "fafnar") then
		npcHandler:say('Fafnar is the scorching sun. She observes the creation with hate and jealousy.', cid)
		return true
	elseif msgcontains(msg, "brog") then
		npcHandler:say('Brog, the raging one, is the great destroyer. The berserk of darkness.', cid)
		return true
	elseif msgcontains(msg, "urgith") then
		npcHandler:say('The bonemaster Urgith is the lord of the undead and keeper of the damned souls.', cid)
		return true
	elseif msgcontains(msg, "archdemons") then
		npcHandler:say('The demons are followers of Zathroth. The cruelest are known as the ruthless seven.', cid)
		return true
	elseif msgcontains(msg, "ruthless seven") then
		npcHandler:say('I dont want to talk about that subject!', cid)
		return true
	end

	return false
end

npcHandler:setCallback(CALLBACK_MESSAGE_DEFAULT, creatureSayCallback)

npcHandler:setMessage(MESSAGE_GREET, 'Welcome in the name of the gods, pilgrim |PLAYERNAME|!')
npcHandler:setMessage(MESSAGE_FAREWELL, 'Good bye, |PLAYERNAME|. May the gods be with you to guard and guide you, my child!')


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
