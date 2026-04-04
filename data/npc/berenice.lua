-- Berenice - Converted from XML to Lua NpcType
-- Original XML: data/npc/Berenice.xml
-- Original Script: data/npc/scripts/Berenice.lua

local npcName = "Berenice"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a berenice")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(2000)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 140, lookHead = 5, lookBody = 87, lookLegs = 104, lookFeet = 106})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)


local function creatureSayCallback(cid, type, msg)
	if not npcHandler:isFocused(cid) then
		return false
	end

	local player = Player(cid)

	if msgcontains(msg, "mission") then
		if player:getStorageValue(Storage.ExplorerSociety.CalassaQuest) == 2 then
			npcHandler:say("OH! So you have safely returned from Calassa! Congratulations, were you able to retrieve the logbook?", cid)
			npcHandler.topic[cid] = 5
		elseif player:getStorageValue(Storage.ExplorerSociety.TheOrcPowder) > 34 and player:getStorageValue(Storage.ExplorerSociety.QuestLine) > 34 then
			npcHandler:say("The most important mission we currently have is an expedition to {Calassa}.", cid)
			npcHandler.topic[cid] = 1
		end
	elseif msgcontains(msg, "calassa") then
		if npcHandler.topic[cid] == 1 and player:getStorageValue(Storage.ExplorerSociety.CalassaQuest) < 1 then
			npcHandler:say("Ah! So you have heard about our special mission to investigate the Quara race in their natural surrounding! Would you like to know more about it?", cid)
			npcHandler.topic[cid] = 2
		elseif npcHandler.topic[cid] == 4 then
			npcHandler:say("Captain Max will bring you to Calassa whenever you are ready. Please try to retrieve the missing logbook which must be in one of the sunken shipwrecks.", cid)
			
			npcHandler.topic[cid] = 0
		elseif player:getStorageValue(Storage.ExplorerSociety.CalassaQuest) == 2 then
			npcHandler:say("OH! So you have safely returned from Calassa! Congratulations, were you able to retrieve the logbook?", cid)
			npcHandler.topic[cid] = 5
		end
	elseif msgcontains(msg, "yes") then
		if npcHandler.topic[cid] == 2 then
			npcHandler:say({
				"Since you have already proved to be a valuable member of our society, I will happily entrust you with this mission, but there are a few things which you need to know, so listen carefully. ...",
				"Calassa is an underwater settlement, so you are in severe danger of drowning unless you are well-prepared. ...",
				"We have developed a new device called 'Helmet of the Deep' which will enable you to breathe even in the depths of the ocean. ...",
				"I will instruct Captain Max to bring you to Calassa and to lend one of these helmets to you. These helmets are very valuable, so there is a deposit of 5000 gold pieces on it. ...",
				"While in Calassa, do not take the helmet off under any circumstances. If you have any questions, don't hesitate to ask Captain Max. ...",
				"Your mission there, apart from observing the Quara, is to retrieve a special logbook from one of the shipwrecks buried there. ...",
				"One of our last expeditions there failed horribly and the ship sank, but we still do not know the exact reason. ...",
				"If you could retrieve the logbook, we'd finally know what happened. Have you understood your task and are willing to take this risk?"
			}, cid)
			npcHandler.topic[cid] = 3
		elseif npcHandler.topic[cid] == 3 then
			player:setStorageValue(Storage.ExplorerSociety.CalassaQuest, 1)
			npcHandler:say("Excellent! I will immediately inform Captain Max to bring you to {Calassa} whenever you are ready. Don't forget to make thorough preparations!", cid)
			npcHandler.topic[cid] = 4
		elseif npcHandler.topic[cid] == 5 then
			if player:removeItem(6124, 1) then
				player:setStorageValue(Storage.ExplorerSociety.CalassaQuest, 3)
				npcHandler:say("Yes! That's the logbook! However... it seems that the water has already destroyed many of the pages. This is not your fault though, you did your best. Thank you!", cid)
				npcHandler.topic[cid] = 0
			end
		end
	end
	return true
end

npcHandler:setCallback(CALLBACK_MESSAGE_DEFAULT, creatureSayCallback)


-- Shop items (from XML parameters)
local shopItems = {
    {id = 6087, buy = 0, sell = 100, subType = 0, name = "first verse of the hymn"},
    {id = 6088, buy = 0, sell = 250, subType = 0, name = "second verse of the hymn"},
    {id = 6089, buy = 0, sell = 400, subType = 0, name = "third verse of the hymn"},
    {id = 6090, buy = 0, sell = 800, subType = 0, name = "fourth verse of the hymn"},
    {id = 5022, buy = 80, sell = 0, subType = 0, name = "orichalcum pearl"},
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

npcType:eventType(NPCS_EVENT_SELLITEM)
npcType:onSellItem(function(npc, player, itemId, subType, amount, ignoreEquipped)
    local shopItem = getShopItem(itemId, subType, false)
    if not shopItem or shopItem.sell <= 0 then return false end
    local totalPrice = amount * shopItem.sell
    local itemName = shopItem.name or ItemType(itemId):getName()
    
    local itemSubType = -1
    if ItemType(itemId):isFluidContainer() then
        itemSubType = subType
    end
    
    if doPlayerSellItem(player:getId(), itemId, amount, totalPrice, itemSubType, ignoreEquipped) then
        player:sendTextMessage(MESSAGE_INFO_DESCR, "Sold " .. amount .. "x " .. shopItem.name .. " for " .. (amount * shopItem.sell) .. " gold.")
        return true
    end
    player:sendCancelMessage("You do not have this object.")
    return false
end)

npcHandler:addModule(FocusModule:new())
npcType:register()
