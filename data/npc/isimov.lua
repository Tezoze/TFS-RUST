-- Isimov - Converted from XML to Lua NpcType
-- Original XML: data/npc/Isimov.xml
-- Original Script: data/npc/scripts/Isimov.lua

local npcName = "Isimov"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a isimov")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(2000)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 160, lookHead = 116, lookBody = 38, lookLegs = 19, lookFeet = 95})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)


-- Twist of Fate
local blessKeyword = keywordHandler:addKeyword({'twist of fate'}, StdModule.say, {npcHandler = npcHandler,
		text = {
			'This is a special blessing I can bestow upon you once you have obtained at least one of the other blessings and which functions a bit differently. ...',
			'It only works when you\'re killed by other adventurers, which means that at least half of the damage leading to your death was caused by others, not by monsters or the environment. ...',
			'The {twist of fate} will not reduce the death penalty like the other blessings, but instead prevent you from losing your other blessings as well as the amulet of loss, should you wear one. It costs the same as the other blessings. ...',
			'Would you like to receive that protection for a sacrifice of |PVPBLESSCOST| gold, child?'
		}
	})
	blessKeyword:addChildKeyword({'yes'}, StdModule.bless, {npcHandler = npcHandler, text = 'So receive the protection of the twist of fate, pilgrim.', cost = '|PVPBLESSCOST|', bless = 6})
	blessKeyword:addChildKeyword({''}, StdModule.say, {npcHandler = npcHandler, text = 'Fine. You are free to decline my offer.', reset = true})

-- Adventurer Stone
keywordHandler:addKeyword({'adventurer stone'}, StdModule.say, {npcHandler = npcHandler, text = 'Keep your adventurer\'s stone well.'}, function(player) return player:getItemById(18559, true) end)

local stoneKeyword = keywordHandler:addKeyword({'adventurer stone'}, StdModule.say, {npcHandler = npcHandler, text = 'Ah, you want to replace your adventurer\'s stone for free?'}, function(player) return player:getStorageValue(Storage.AdventurersGuild.FreeStone.Isimov) ~= 1 end)
	stoneKeyword:addChildKeyword({'yes'}, StdModule.say, {npcHandler = npcHandler, text = 'Here you are. Take care.', reset = true}, nil, function(player) player:addItem(18559, 1) player:setStorageValue(Storage.AdventurersGuild.FreeStone.Isimov, 1) end)
	stoneKeyword:addChildKeyword({''}, StdModule.say, {npcHandler = npcHandler, text = 'No problem.', reset = true})

local stoneKeyword = keywordHandler:addKeyword({'adventurer stone'}, StdModule.say, {npcHandler = npcHandler, text = 'Ah, you want to replace your adventurer\'s stone for 30 gold?'})
	stoneKeyword:addChildKeyword({'yes'}, StdModule.say, {npcHandler = npcHandler, text = 'Here you are. Take care.', reset = true},
		function(player) return player:getMoney() + player:getBankBalance() >= 30 end,
		function(player) if player:removeMoneyNpc(30) then player:addItem(18559, 1) end end
	)
	stoneKeyword:addChildKeyword({'yes'}, StdModule.say, {npcHandler = npcHandler, text = 'Sorry, you don\'t have enough money.', reset = true})
	stoneKeyword:addChildKeyword({''}, StdModule.say, {npcHandler = npcHandler, text = 'No problem.', reset = true})

local function creatureSayCallback(cid, type, msg)
	if not npcHandler:isFocused(cid) then
		return false
	end

	local player = Player(cid)
	if not player then
		return false
	end

	-- Wooden Stake
	if msgcontains(msg, "stake") then
	local stakeStorage = player:getStorageValue(Storage.FriendsAndTraders.TheBlessedStake)

	if stakeStorage == 5 and player:getItemCount(5941) == 0 then
		npcHandler:say('I think you have forgotten to bring your stake, kid.', cid)
	elseif stakeStorage == 5 then
		npcHandler:say('Yes, I was informed what to do. Are you prepared to receive my line of the prayer?', cid)
		npcHandler.topic[cid] = 5
	elseif stakeStorage == 6 then
		npcHandler:say('You should visit Amanda in Edron now, kid.', cid)
	elseif stakeStorage > 6 then
		npcHandler:say('You already received my line of the prayer.', cid)
	else
		npcHandler:say('A blessed stake? That\'s a strange request. Maybe Quentin knows more, he is one of the oldest monks after all.', cid)
	end
	return true

-- Handle yes/no responses for stake quest
elseif npcHandler.topic[cid] == 5 and msgcontains(msg, "yes") then
	if player:getItemCount(5941) > 0 then
		npcHandler:say('So receive my prayer: \'Unclean spirits shall be repelled\'. Now, bring your stake to Amanda in Edron for the next line of the prayer. I will inform her what to do.', cid)
		player:setStorageValue(Storage.FriendsAndTraders.TheBlessedStake, 6)
		player:getPosition():sendMagicEffect(CONST_ME_MAGIC_BLUE)
	else
		npcHandler:say('You don\'t even have that strange stake with you.', cid)
	end
	npcHandler.topic[cid] = 0
	return true
elseif npcHandler.topic[cid] == 5 and msgcontains(msg, "no") then
	npcHandler:say('I will wait for you.', cid)
	npcHandler.topic[cid] = 0
	return true
end

return false
end

-- Healing
local function addHealKeyword(text, condition, effect)
	keywordHandler:addKeyword({'heal'}, StdModule.say, {npcHandler = npcHandler, text = text},
		function(player) return player:hasCondition(condition) end,
		function(player)
			player:removeCondition(condition)
			player:getPosition():sendMagicEffect(effect)
		end
	)
end

addHealKeyword('You are burning. Let me quench those flames.', CONDITION_FIRE, CONST_ME_MAGIC_GREEN)
addHealKeyword('You are poisoned. Let me soothe your pain.', CONDITION_POISON, CONST_ME_MAGIC_RED)
addHealKeyword('You are electrified, my child. Let me help you to stop trembling.', CONDITION_ENERGY, CONST_ME_MAGIC_GREEN)

keywordHandler:addKeyword({'heal'}, StdModule.say, {npcHandler = npcHandler, text = 'You are hurt, my child. I will heal your wounds.'},
	function(player) return player:getHealth() < 40 end,
	function(player)
		local health = player:getHealth()
		if health < 40 then player:addHealth(40 - health) end
		player:getPosition():sendMagicEffect(CONST_ME_MAGIC_GREEN)
	end
)
keywordHandler:addKeyword({'heal'}, StdModule.say, {npcHandler = npcHandler, text = 'You aren\'t looking that bad. Sorry, I can\'t help you. But if you are looking for additional protection you should go on the {pilgrimage} of ashes or get the protection of the {twist of fate} here.'})

-- Basic
keywordHandler:addKeyword({'pilgrimage'}, StdModule.say, {npcHandler = npcHandler, text = 'Whenever you receive a lethal wound, your vital force is damaged and there is a chance that you lose some of your equipment. With every single of the five {blessings} you have, this damage and chance of loss will be reduced.'})
keywordHandler:addKeyword({'blessings'}, StdModule.say, {npcHandler = npcHandler, text = 'There are five blessings available in five sacred places: the {spiritual} shielding, the spark of the {phoenix}, the {embrace} of Tibia, the fire of the {suns} and the wisdom of {solitude}. Additionally, you can receive the {twist of fate} here.'})
keywordHandler:addKeyword({'spiritual'}, StdModule.say, {npcHandler = npcHandler, text = 'I see you received the spiritual shielding in the whiteflower temple south of Thais.'}, function(player) return player:hasBlessing(1) end)
keywordHandler:addAliasKeyword({'shield'})
keywordHandler:addKeyword({'embrace'}, StdModule.say, {npcHandler = npcHandler, text = 'I can sense that the druids north of Carlin have provided you with the Embrace of Tibia.'}, function(player) return player:hasBlessing(2) end)
keywordHandler:addKeyword({'suns'}, StdModule.say, {npcHandler = npcHandler, text = 'I can see you received the blessing of the two suns in the suntower near Ab\'Dendriel.'}, function(player) return player:hasBlessing(3) end)
keywordHandler:addAliasKeyword({'fire'})
keywordHandler:addKeyword({'phoenix'}, StdModule.say, {npcHandler = npcHandler, text = 'I can sense that the spark of the phoenix already was given to you by the dwarven priests of earth and fire in Kazordoon.'}, function(player) return player:hasBlessing(4) end)
keywordHandler:addAliasKeyword({'spark'})
keywordHandler:addKeyword({'solitude'}, StdModule.say, {npcHandler = npcHandler, text = 'I can sense you already talked to the hermit Eremo on the isle of Cormaya and received this blessing.'}, function(player) return player:hasBlessing(5) end)
keywordHandler:addAliasKeyword({'wisdom'})
keywordHandler:addKeyword({'spiritual'}, StdModule.say, {npcHandler = npcHandler, text = 'You can ask for the blessing of spiritual shielding in the whiteflower temple south of Thais.'})
keywordHandler:addAliasKeyword({'shield'})
keywordHandler:addKeyword({'embrace'}, StdModule.say, {npcHandler = npcHandler, text = 'The druids north of Carlin will provide you with the embrace of Tibia.'})
keywordHandler:addKeyword({'suns'}, StdModule.say, {npcHandler = npcHandler, text = 'You can ask for the blessing of the two suns in the suntower near Ab\'Dendriel.'})
keywordHandler:addAliasKeyword({'fire'})
keywordHandler:addKeyword({'phoenix'}, StdModule.say, {npcHandler = npcHandler, text = 'The spark of the phoenix is given by the dwarven priests of earth and fire in Kazordoon.'})
keywordHandler:addAliasKeyword({'spark'})
keywordHandler:addKeyword({'solitude'}, StdModule.say, {npcHandler = npcHandler, text = 'Talk to the hermit Eremo on the isle of Cormaya about this blessing.'})
keywordHandler:addAliasKeyword({'wisdom'})

npcHandler:setCallback(CALLBACK_MESSAGE_DEFAULT, creatureSayCallback)

npcHandler:setMessage(MESSAGE_GREET, 'Welcome, young |PLAYERNAME|! If you are heavily wounded or poisoned, I can {heal} you for free.')
npcHandler:setMessage(MESSAGE_WALKAWAY, 'Remember: If you are heavily wounded or poisoned, I can heal you for free.')
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
