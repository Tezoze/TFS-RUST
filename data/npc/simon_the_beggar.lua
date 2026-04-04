-- Simon The Beggar - Converted from XML to Lua NpcType
-- Original XML: data/npc/Simon The Beggar.xml
-- Original Script: data/npc/scripts/Simon the Beggar.lua

local npcName = "Simon The Beggar"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a simon the beggar")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(2000)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 153, lookHead = 116, lookBody = 123, lookLegs = 123, lookFeet = 40, lookAddons = 3})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)


local voices = {
	{ text = 'Alms! Alms for the poor!' },
	{ text = 'Sir, Ma\'am, have a gold coin to spare?' },
	{ text = 'I need help! Please help me!' }
}

npcHandler:addModule(VoiceModule:new(voices))

function BeggarFirst(cid, message, keywords, parameters, node)
	if not npcHandler:isFocused(cid) then
		return false
	end

	local player = Player(cid)
	if player:isPremium() then
		if player:getStorageValue(Storage.OutfitQuest.BeggarFirstAddon) == -1 then
			if player:getItemCount(5883) >= 100 and player:getMoney() + player:getBankBalance() >= 20000 then
				if player:removeItem(5883, 100) and player:removeMoneyNpc(20000) then
					npcHandler:say("Ah, right! The beggar beard or beggar dress! Here you go.", cid)
					player:getPosition():sendMagicEffect(CONST_ME_MAGIC_BLUE)
					player:setStorageValue(Storage.OutfitQuest.BeggarFirstAddon, 1)
					player:addOutfitAddon(153, 1)
					player:addOutfitAddon(157, 1)
				end
			else
				npcHandler:say("You do not have all the required items.", cid)
			end
		else
			npcHandler:say("It seems you already have this addon, don't you try to mock me son!", cid)
		end
	end
end

function BeggarSecond(cid, message, keywords, parameters, node)
	if not npcHandler:isFocused(cid) then
		return false
	end

	local player = Player(cid)
	if player:isPremium() then
		if player:getStorageValue(Storage.OutfitQuest.BeggarSecondAddon) == -1 then
			if player:getItemCount(6107) >= 1 then
				if player:removeItem(6107, 1) then
					npcHandler:say("Ah, right! The beggar staff! Here you go.", cid)
					player:getPosition():sendMagicEffect(CONST_ME_MAGIC_BLUE)
					player:setStorageValue(Storage.OutfitQuest.BeggarSecondAddon, 1)
					player:addOutfitAddon(153, 2)
					player:addOutfitAddon(157, 2)
				end
			else
				npcHandler:say("You do not have all the required items.", cid)
			end
		else
			npcHandler:say("It seems you already have this addon, don't you try to mock me son!", cid)
		end
	end
end

local function creatureSayCallback(cid, type, msg)
	if not npcHandler:isFocused(cid) then
		return false
	end

	local player = Player(cid)

	if msgcontains(msg, 'cookie') then
		if player:getStorageValue(Storage.WhatAFoolishQuest.Questline) == 31
				and player:getStorageValue(Storage.WhatAFoolishQuest.CookieDelivery.SimonTheBeggar) ~= 1 then
			npcHandler:say('Have you brought a cookie for the poor?', cid)
			npcHandler.topic[cid] = 1
		end
	elseif msgcontains(msg, 'help') then
		npcHandler:say('I need gold. Can you spare 100 gold pieces for me?', cid)
		npcHandler.topic[cid] = 2
	elseif msgcontains(msg, 'yes') then
		if npcHandler.topic[cid] == 1 then
			if not player:removeItem(8111, 1) then
				npcHandler:say('You have no cookie that I\'d like.', cid)
				npcHandler.topic[cid] = 0
				return true
			end

			player:setStorageValue(Storage.WhatAFoolishQuest.CookieDelivery.SimonTheBeggar, 1)
			if player:getCookiesDelivered() == 10 then
				player:addAchievement('Allow Cookies?')
			end

			Npc():getPosition():sendMagicEffect(CONST_ME_GIFT_WRAPS)
			npcHandler:say('Well, it\'s the least you can do for those who live in dire poverty. A single cookie is a bit less than I\'d expected, but better than ... WHA ... WHAT?? MY BEARD! MY PRECIOUS BEARD! IT WILL TAKE AGES TO CLEAR IT OF THIS CONFETTI!', cid)
			npcHandler:releaseFocus(cid)
			npcHandler:resetNpc(cid)
		elseif npcHandler.topic[cid] == 2 then
			if not player:removeMoneyNpc(100) then
				npcHandler:say('You haven\'t got enough money for me.', cid)
				npcHandler.topic[cid] = 0
				return true
			end

			npcHandler:say('Thank you very much. Can you spare 500 more gold pieces for me? I will give you a nice hint.', cid)
			npcHandler.topic[cid] = 3
		elseif npcHandler.topic[cid] == 3 then
			if not player:removeMoneyNpc(500) then
				npcHandler:say('Sorry, that\'s not enough.', cid)
				npcHandler.topic[cid] = 0
				return true
			end

			npcHandler:say('That\'s great! I have stolen something from Dermot. You can buy it for 200 gold. Do you want to buy it?', cid)
			npcHandler.topic[cid] = 4
		elseif npcHandler.topic[cid] == 4 then
			if not player:removeMoneyNpc(200) then
				npcHandler:say('Pah! I said 200 gold. You don\'t have that much.', cid)
				npcHandler.topic[cid] = 0
				return true
			end

			local key = player:addItem(2087, 1)
			if key then
				key:setActionId(3940)
			end
			npcHandler:say('Now you own the hot key.', cid)
			npcHandler.topic[cid] = 0
		end
	elseif msgcontains(msg, 'no') and npcHandler.topic[cid] ~= 0 then
		if npcHandler.topic[cid] == 1 then
			npcHandler:say('I see.', cid)
		elseif npcHandler.topic[cid] == 2 then
			npcHandler:say('Hmm, maybe next time.', cid)
		elseif npcHandler.topic[cid] == 3 then
			npcHandler:say('It was your decision.', cid)
		elseif npcHandler.topic[cid] == 4 then
			npcHandler:say('Ok. No problem. I\'ll find another buyer.', cid)
		end
		npcHandler.topic[cid] = 0
	end
	return true
end

node1 = keywordHandler:addKeyword({'addon'}, StdModule.say, {npcHandler = npcHandler, text = 'For the small fee of 20000 gold pieces I will help you mix this potion. Just bring me 100 pieces of ape fur, which are necessary to create this potion. ...Do we have a deal?'})
node1:addChildKeyword({'yes'}, BeggarSecond, {})
node1:addChildKeyword({'no'}, StdModule.say, {npcHandler = npcHandler, text = 'Alright then. Come back when you got all neccessary items.', reset = true})

node2 = keywordHandler:addKeyword({'dress'}, StdModule.say, {npcHandler = npcHandler, text = 'For the small fee of 20000 gold pieces I will help you mix this potion. Just bring me 100 pieces of ape fur, which are necessary to create this potion. ...Do we have a deal?'})
node2:addChildKeyword({'yes'}, BeggarFirst, {})
node2:addChildKeyword({'no'}, StdModule.say, {npcHandler = npcHandler, text = 'Alright then. Come back when you got all neccessary items.', reset = true})

node3 = keywordHandler:addKeyword({'staff'}, StdModule.say, {npcHandler = npcHandler, text = 'To get beggar staff you need to give me simon the beggar\'s staff. Do you have it with you?'})
node3:addChildKeyword({'yes'}, BeggarSecond, {})
node3:addChildKeyword({'no'}, StdModule.say, {npcHandler = npcHandler, text = 'Alright then. Come back when you got all neccessary items.', reset = true})

node4 = keywordHandler:addKeyword({'outfit'}, StdModule.say, {npcHandler = npcHandler, text = 'For the small fee of 20000 gold pieces I will help you mix this potion. Just bring me 100 pieces of ape fur, which are necessary to create this potion. ...Do we have a deal?'})
node4:addChildKeyword({'yes'}, BeggarFirst, {})
node4:addChildKeyword({'no'}, StdModule.say, {npcHandler = npcHandler, text = 'Alright then. Come back when you got all neccessary items.', reset = true})


npcHandler:setMessage(MESSAGE_GREET, "Hello |PLAYERNAME|. I am a poor man. Please help me.")
npcHandler:setMessage(MESSAGE_FAREWELL, "Have a nice day.")
npcHandler:setMessage(MESSAGE_WALKAWAY, "Have a nice day.")

npcHandler:setCallback(CALLBACK_MESSAGE_DEFAULT, creatureSayCallback)


-- Shop items (from XML parameters)
local shopItems = {
    {id = 2554, buy = 50, sell = 0, subType = 0, name = "shovel"},
}

-- Helper function to find shop item by id and subType (for fluid containers)
local function getShopItem(itemId, subType, isBuying)
    local itemType = ItemType(itemId)
    if itemType:isFluidContainer() then
        for _, item in ipairs(shopItems) do
            if item.id == itemId and item.subType == subType then
                return item
            end
        end
    end
    -- For non-fluid items, find the entry that matches the operation
    for _, item in ipairs(shopItems) do
        if item.id == itemId then
            if isBuying and item.buy > 0 then
                return item
            elseif not isBuying and item.sell > 0 then
                return item
            end
        end
    end
    -- Fallback to first match
    for _, item in ipairs(shopItems) do
        if item.id == itemId then
            return item
        end
    end
    return nil
end

local function openTradeWindow(cid, message, keywords, parameters, node)
    if not npcHandler:isFocused(cid) then return false end
    local player = Player(cid)
    if not player then return false end
    local npc = Npc(getNpcCid())
    local shopList = {}
    for _, item in ipairs(shopItems) do
        table.insert(shopList, {id = item.id, buy = item.buy, sell = item.sell, subType = item.subType or 0, name = item.name})
    end
    npc:openShopWindow(player, shopList, function() return true end, function() return true end)
    npcHandler:say('Take all the time you need to browse my wares.', cid)
    return true
end
keywordHandler:addKeyword({'trade'}, openTradeWindow, {npcHandler = npcHandler})


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

npcType:eventType(NPCS_EVENT_BUYITEM)
npcType:onBuyItem(function(npc, player, itemId, subType, amount, ignoreCap, inBackpacks)
    local shopItem = getShopItem(itemId, subType, true)
    if not shopItem or shopItem.buy <= 0 then return false end
    local totalCost = amount * shopItem.buy
    if player:getTotalMoney() < totalCost then
        player:sendCancelMessage("You don't have enough money.")
        return false
    end
    local itemSubType = shopItem.subType or 1
    local bought = doNpcSellItem(player:getId(), itemId, amount, itemSubType, ignoreCap, inBackpacks, ITEM_BACKPACK)
    if bought == 0 then
        player:sendCancelMessage("You do not have enough capacity.")
        return false
    end
    player:removeTotalMoney(bought * shopItem.buy)
    player:sendTextMessage(MESSAGE_INFO_DESCR, "Bought " .. bought .. "x " .. shopItem.name .. " for " .. (bought * shopItem.buy) .. " gold.")
    return true
end)

npcHandler:addModule(FocusModule:new())
npcType:register()
