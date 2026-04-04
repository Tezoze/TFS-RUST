-- Amanda - Converted from XML to Lua NpcType
-- Original XML: data/npc/Amanda.xml
-- Original Script: data/npc/scripts/Amanda.lua

local npcName = "Amanda"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a amanda")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(2000)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 138, lookHead = 96, lookBody = 95, lookFeet = 114})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)


-- Main callback for custom healing (expanded)
local function creatureSayCallback(cid, type, msg)
    if not npcHandler:isFocused(cid) then return false end
    local player = Player(cid)
    local pvpBlessCost = StdModule.calculateRegularBlessingCost(player:getLevel())

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
        elseif player:getCondition(CONDITION_PARALYZE) then
            npcHandler:say("You are paralyzed. Let me cure you.", cid)
            player:removeCondition(CONDITION_PARALYZE)
            player:getPosition():sendMagicEffect(CONST_ME_MAGIC_BLUE)
            healed = true
        elseif player:getCondition(CONDITION_DROWN) then
            npcHandler:say("You are drowning. Let me help you.", cid)
            player:removeCondition(CONDITION_DROWN)
            player:getPosition():sendMagicEffect(CONST_ME_MAGIC_BLUE)
            healed = true
        elseif player:getCondition(CONDITION_FREEZING) then
            npcHandler:say("You are freezing! Let me warm you up.", cid)
            player:removeCondition(CONDITION_FREEZING)
            player:getPosition():sendMagicEffect(CONST_ME_MAGIC_BLUE)
            healed = true
        elseif player:getCondition(CONDITION_BLEEDING) then
            npcHandler:say("You are bleeding! Let me stop that.", cid)
            player:removeCondition(CONDITION_BLEEDING)
            player:getPosition():sendMagicEffect(CONST_ME_MAGIC_BLUE)
            healed = true
        elseif player:getCondition(CONDITION_DAZZLED) then
            npcHandler:say("You are dazzled! Do not mess with holy creatures anymore!", cid)
            player:removeCondition(CONDITION_DAZZLED)
            player:getPosition():sendMagicEffect(CONST_ME_MAGIC_BLUE)
            healed = true
        elseif player:getCondition(CONDITION_CURSED) then
            npcHandler:say("You are cursed! I will remove it.", cid)
            player:removeCondition(CONDITION_CURSED)
            player:getPosition():sendMagicEffect(CONST_ME_MAGIC_BLUE)
            healed = true
        elseif player:getHealth() < 65 then
            npcHandler:say("You are looking really bad. Let me heal your wounds.", cid)
            local health = player:getHealth()
            if health < 65 then player:addHealth(65 - health) end
            player:getPosition():sendMagicEffect(CONST_ME_MAGIC_BLUE)
            healed = true
        end
        if not healed then
            npcHandler:say("You aren't looking that bad. Sorry, I can't help you. But if you are looking for additional protection you should go on the {pilgrimage} of ashes or get the protection of the {twist of fate} here.", cid)
        end
        return true

	-- Wooden Stake
	elseif msgcontains(msg, "stake") then
		local stakeStorage = player:getStorageValue(Storage.FriendsAndTraders.TheBlessedStake)

		if stakeStorage == 6 and player:getItemCount(5941) == 0 then
			npcHandler:say('I think you have forgotten to bring your stake, my child.', cid)
		elseif stakeStorage == 6 then
			npcHandler:say('Yes, I was informed what to do. Are you prepared to receive my line of the prayer?', cid)
			npcHandler.topic[cid] = 6
		elseif stakeStorage == 7 then
			npcHandler:say('You should visit Kasmir in Darashia now, my child.', cid)
		elseif stakeStorage > 7 then
			npcHandler:say('You already received my line of the prayer.', cid)
		else
			npcHandler:say('A blessed stake? That\'s a strange request. Maybe Quentin knows more, he is one of the oldest monks after all.', cid)
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
		npcHandler.topic[cid] = 7
		return true
	end

	-- Handle yes/no responses for stake quest and twist of fate
	if npcHandler.topic[cid] == 6 and msgcontains(msg, "yes") then
		if player:getItemCount(5941) > 0 then
			npcHandler:say('So receive my prayer: \'Wicked curses shall be broken\'. Now, bring your stake to Kasmir in Darashia for the next line of the prayer. I will inform him what to do.', cid)
			player:setStorageValue(Storage.FriendsAndTraders.TheBlessedStake, 7)
			player:getPosition():sendMagicEffect(CONST_ME_MAGIC_BLUE)
		else
			npcHandler:say('You don\'t even have that strange stake with you.', cid)
		end
		npcHandler.topic[cid] = 0
		return true
	elseif npcHandler.topic[cid] == 6 and msgcontains(msg, "no") then
		npcHandler:say('I will wait for you.', cid)
		npcHandler.topic[cid] = 0
		return true
	elseif npcHandler.topic[cid] == 7 and msgcontains(msg, "yes") then
		if player:hasBlessing(6) then
			npcHandler:say("Gods have already blessed you with this blessing!", cid)
		elseif not player:hasAnyBlessing() then
			npcHandler:say("You need at least one regular blessing first.", cid)
		elseif not player:removeTotalMoney(pvpBlessCost) then
			npcHandler:say("You don't have enough money for blessing.", cid)
		else
			player:addBlessing(6)
			npcHandler:say("So receive the protection of the twist of fate, pilgrim.", cid)
			player:getPosition():sendMagicEffect(CONST_ME_MAGIC_BLUE)
		end
		npcHandler.topic[cid] = 0
		return true
	elseif npcHandler.topic[cid] == 7 and msgcontains(msg, "no") then
		npcHandler:say('Fine. You are free to decline my offer.', cid)
		npcHandler.topic[cid] = 0
		return true
	end

	return false
end

npcHandler:setCallback(CALLBACK_MESSAGE_DEFAULT, creatureSayCallback)

-- Adventurer Stone
keywordHandler:addKeyword({'adventurer stone'}, StdModule.say, {npcHandler = npcHandler, text = 'Keep your adventurer\'s stone well.'}, function(player) return player:getItemCount(18559) > 0 end)

local stoneKeyword1 = keywordHandler:addKeyword({'adventurer stone'}, StdModule.say, {npcHandler = npcHandler, text = 'Ah, you want to replace your adventurer\'s stone for free?'}, function(player) return player:getStorageValue(Storage.AdventurersGuild.FreeStone.Amanda) ~= 1 end)
stoneKeyword1:addChildKeyword({'yes'}, StdModule.say, {npcHandler = npcHandler, text = 'Here you are. Take care.', reset = true}, nil, function(player) player:addItem(18559, 1) player:setStorageValue(Storage.AdventurersGuild.FreeStone.Amanda, 1) end)
stoneKeyword1:addChildKeyword({''}, StdModule.say, {npcHandler = npcHandler, text = 'No problem.', reset = true})

local stoneKeyword2 = keywordHandler:addKeyword({'adventurer stone'}, StdModule.say, {npcHandler = npcHandler, text = 'Ah, you want to replace your adventurer\'s stone for 30 gold?'})
stoneKeyword2:addChildKeyword({'yes'}, StdModule.say, {npcHandler = npcHandler, text = 'Here you are. Take care.', reset = true},
    function(player) return player:getMoney() + player:getBankBalance() >= 30 end,
    function(player) player:removeMoneyNpc(30) player:addItem(18559, 1) end
)
stoneKeyword2:addChildKeyword({'yes'}, StdModule.say, {npcHandler = npcHandler, text = 'Sorry, you don\'t have enough money.', reset = true})
stoneKeyword2:addChildKeyword({''}, StdModule.say, {npcHandler = npcHandler, text = 'No problem.', reset = true})


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
