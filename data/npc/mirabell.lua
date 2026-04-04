-- Mirabell - Converted from XML to Lua NpcType
-- Original XML: data/npc/Mirabell.xml
-- Original Script: data/npc/scripts/Mirabell.lua

local npcName = "Mirabell"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a mirabell")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(2000)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 136, lookHead = 96, lookBody = 12, lookLegs = 87, lookFeet = 77})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)


local voices = { {text = 'The Horn of Plenty is always open for tired adventurers.'} }
npcHandler:addModule(VoiceModule:new(voices))

local function creatureSayCallback(cid, type, msg)
	if not npcHandler:isFocused(cid) then
		return false
	end

	local player = Player(cid)

	if msgcontains(msg, 'pies') then
		if player:getStorageValue(Storage.WhatAFoolishQuest.PieBuying) == -1 then
			npcHandler:say('Oh you\'ve heard about my excellent pies, didn\'t you? I am flattered. Unfortunately I\'m completely out of flour. I need 2 portions of flour for one pie. Just tell me when you have enough flour for your pies.', cid)
			return true
		end

		npcHandler:say('For 12 pies this is 240 gold. Do you want to buy them?', cid)
		npcHandler.topic[cid] = 2
	elseif msgcontains(msg, 'flour') then
		npcHandler:say('Do you bring me the flour needed for your pies?', cid)
		npcHandler.topic[cid] = 1
	elseif msgcontains(msg, 'yes') then
		if npcHandler.topic[cid] == 1 then
			if not player:removeItem(2692, 24) then
				npcHandler:say('I think you are confusing the dust in your pockets with flour. You certainly do not have enough flour for 12 pies.', cid)
				npcHandler.topic[cid] = 0
				return true
			end

			player:setStorageValue(Storage.WhatAFoolishQuest.PieBuying, player:getStorageValue(Storage.WhatAFoolishQuest.PieBuying) + 1)
			npcHandler:say('Excellent. Now I can start baking the pies. As you helped me, I will make you a good price for them.', cid)
			npcHandler.topic[cid] = 0
		elseif npcHandler.topic[cid] == 2 then
			if not player:removeMoneyNpc(240) then
				npcHandler:say('You don\'t have enough money, don\'t try to fool me.', cid)
				npcHandler.topic[cid] = 0
				return true
			end

			player:addItem(7484, 1)
			player:setStorageValue(Storage.WhatAFoolishQuest.PieBuying, player:getStorageValue(Storage.WhatAFoolishQuest.PieBuying) - 1)
			player:setStorageValue(Storage.WhatAFoolishQuest.PieBoxTimer, os.time() + 1200) -- 20 minutes to deliver
			npcHandler:say({
				'Here they are. Wait! Two things you should know: Firstly, they won\'t last long in the sun so you better get them to their destination as quickly as possible ...',
				'Secondly, since my pies are that delicious it is forbidden to leave the town with them. We can\'t afford to attract more tourists to Edron.'
			}, cid)
			npcHandler.topic[cid] = 0
		end
	elseif msgcontains(msg, 'no') then
		if npcHandler.topic[cid] == 1 then
			npcHandler:say('Without flour I can\'t do anything, sorry.', cid)
		elseif npcHandler.topic[cid] == 2 then
			npcHandler:say('What are you? Some kind of fool?', cid)
		end
		npcHandler.topic[cid] = 0
	end

	return true
end

keywordHandler:addKeyword({'drink'}, StdModule.say, {npcHandler = npcHandler, text = 'I can offer you beer, wine, lemonade and water. If you\'d like to see my offers, ask me for a {trade}.'})
keywordHandler:addKeyword({'food'}, StdModule.say, {npcHandler = npcHandler, text = 'Are you looking for food? I have bread, cheese, ham, and meat. If you\'d like to see my offers, ask me for a {trade}.'})

npcHandler:setMessage(MESSAGE_GREET, "Welcome to the Horn of Plenty, |PLAYERNAME|. Sit down, have a {drink} or some {food}!")
npcHandler:setMessage(MESSAGE_FAREWELL, "Come back soon, traveller.")
npcHandler:setMessage(MESSAGE_WALKAWAY, "Come back soon, traveller.")
npcHandler:setMessage(MESSAGE_SENDTRADE, "Of course, take a look at my tasty offers.")

npcHandler:setCallback(CALLBACK_MESSAGE_DEFAULT, creatureSayCallback)


-- Shop items (from XML parameters)
local shopItems = {
    {id = 2012, buy = 2, sell = 0, subType = 5, name = "mug of lemonade"},
    {id = 2689, buy = 4, sell = 0, subType = 0, name = "bread"},
    {id = 2696, buy = 6, sell = 0, subType = 0, name = "cheese"},
    {id = 2671, buy = 8, sell = 0, subType = 0, name = "ham"},
    {id = 2666, buy = 5, sell = 0, subType = 0, name = "meat"},
    {id = 2006, buy = 10, sell = 0, subType = 15, name = "wine"},
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
