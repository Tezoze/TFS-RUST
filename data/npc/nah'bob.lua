-- Nah'bob - Converted from XML to Lua NpcType
-- Original XML: data/npc/Nah'bob.xml
-- Original Script: data/npc/scripts/Nah_Bob.lua

local npcName = "Nah'Bob"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a nah'bob")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(2000)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 80})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)


local function creatureSayCallback(cid, type, msg)
	if not npcHandler:isFocused(cid) then
		return false
	end

	local player = Player(cid)

	if msgcontains(msg, 'cookie') then
		if player:getStorageValue(Storage.WhatAFoolishQuest.Questline) == 31
				and player:getStorageValue(Storage.WhatAFoolishQuest.CookieDelivery.Djinn) ~= 1 then
			npcHandler:say('You brought cookies! How nice of you! Can I have one?', cid)
			npcHandler.topic[cid] = 1
		end
	elseif msgcontains(msg, 'yes') then
		if npcHandler.topic[cid] == 1 then
			if not player:removeItem(8111, 1) then
				npcHandler:say('You have no cookie that I\'d like.', cid)
				npcHandler.topic[cid] = 0
				return true
			end

			player:setStorageValue(Storage.WhatAFoolishQuest.CookieDelivery.Djinn, 1)
			if player:getCookiesDelivered() == 10 then
				player:addAchievement('Allow Cookies?')
			end

			Npc():getPosition():sendMagicEffect(CONST_ME_GIFT_WRAPS)
			npcHandler:say('You see, good deeds like this will ... YOU ... YOU SPAWN OF EVIL! I WILL MAKE SURE THE MASTER LEARNS ABOUT THIS!', cid)
			npcHandler:releaseFocus(cid)
			npcHandler:resetNpc(cid)
		end
	elseif msgcontains(msg, 'no') then
		if npcHandler.topic[cid] == 1 then
			npcHandler:say('I see.', cid)
			npcHandler.topic[cid] = 0
		end
	end
	return true
end

local function onTradeRequest(cid)
	local player = Player(cid)
	if player:getStorageValue(Storage.DjinnWar.MaridFaction.Mission03) ~= 3 then
		npcHandler:say('I\'m sorry, human. But you need Gabel\'s permission to trade with me.', cid)
		return false
	end

	return true
end

npcHandler:setMessage(MESSAGE_GREET, "<Sighs> Another {customer}! I've only just sat down! What is it, |PLAYERNAME|?")
npcHandler:setMessage(MESSAGE_FAREWELL, "Bye now, Neutrala |PLAYERNAME|. Visit old Bob again one day!")
npcHandler:setMessage(MESSAGE_WALKAWAY, "Bye then.")
npcHandler:setMessage(MESSAGE_SENDTRADE, 'At your service, just browse through my wares.')

npcHandler:setCallback(CALLBACK_ONTRADEREQUEST, onTradeRequest)
npcHandler:setCallback(CALLBACK_MESSAGE_DEFAULT, creatureSayCallback)


-- Shop items (from XML parameters)
local shopItems = {
    {id = 7436, buy = 0, sell = 5000, subType = 0, name = "angelic axe"},
    {id = 2656, buy = 0, sell = 10000, subType = 0, name = "blue robe"},
    {id = 2518, buy = 0, sell = 1200, subType = 0, name = "bonelord shield"},
    {id = 2195, buy = 0, sell = 30000, subType = 0, name = "boots of haste"},
    {id = 2413, buy = 0, sell = 500, subType = 0, name = "broadsword"},
    {id = 7412, buy = 0, sell = 18000, subType = 0, name = "butcher's axe"},
    {id = 2487, buy = 0, sell = 12000, subType = 0, name = "crown armor"},
    {id = 2491, buy = 0, sell = 2500, subType = 0, name = "crown helmet"},
    {id = 2488, buy = 0, sell = 12000, subType = 0, name = "crown legs"},
    {id = 2519, buy = 0, sell = 8000, subType = 0, name = "crown shield"},
    {id = 2497, buy = 0, sell = 6000, subType = 0, name = "crusader helmet"},
    {id = 2414, buy = 0, sell = 9000, subType = 0, name = "dragon lance"},
    {id = 2516, buy = 0, sell = 4000, subType = 0, name = "dragon shield"},
    {id = 7854, buy = 0, sell = 1000, subType = 0, name = "earth spike sword"},
    {id = 7868, buy = 0, sell = 1200, subType = 0, name = "earth war hammer"},
    {id = 7869, buy = 0, sell = 1000, subType = 0, name = "energy spike sword"},
    {id = 7883, buy = 0, sell = 1200, subType = 0, name = "energy war hammer"},
    {id = 7744, buy = 0, sell = 1000, subType = 0, name = "fiery spike sword"},
    {id = 7758, buy = 0, sell = 1200, subType = 0, name = "fiery war hammer"},
    {id = 2432, buy = 0, sell = 8000, subType = 0, name = "fire axe"},
    {id = 2392, buy = 0, sell = 4000, subType = 0, name = "fire sword"},
    {id = 7454, buy = 0, sell = 3000, subType = 0, name = "glorious axe"},
    {id = 2515, buy = 0, sell = 2000, subType = 0, name = "guardian shield"},
    {id = 2396, buy = 0, sell = 1000, subType = 0, name = "ice rapier"},
    {id = 7763, buy = 0, sell = 1000, subType = 0, name = "icy spike sword"},
    {id = 7777, buy = 0, sell = 1200, subType = 0, name = "icy war hammer"},
    {id = 2486, buy = 0, sell = 900, subType = 0, name = "noble armor"},
    {id = 2425, buy = 0, sell = 500, subType = 0, name = "obsidian lance"},
    {id = 2539, buy = 0, sell = 16000, subType = 0, name = "phoenix shield"},
    {id = 7410, buy = 0, sell = 20000, subType = 0, name = "queen's sceptre"},
    {id = 2498, buy = 0, sell = 30000, subType = 0, name = "royal helmet"},
    {id = 7451, buy = 0, sell = 10000, subType = 0, name = "shadow sceptre"},
    {id = 2383, buy = 0, sell = 1000, subType = 0, name = "spike sword"},
    {id = 7391, buy = 0, sell = 16000, subType = 0, name = "thaian sword"},
    {id = 2391, buy = 0, sell = 1200, subType = 0, name = "war hammer"},
    {id = 2518, buy = 7000, sell = 0, subType = 0, name = "bonelord shield"},
    {id = 2486, buy = 8000, sell = 0, subType = 0, name = "noble armor"},
    {id = 2425, buy = 3000, sell = 0, subType = 0, name = "obsidian lance"},
    {id = 2383, buy = 8000, sell = 0, subType = 0, name = "spike sword"},
    {id = 2391, buy = 10000, sell = 0, subType = 0, name = "war hammer"},
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
    -- Quest check: must have Gabel's permission (Marid Faction Mission03 = 3)
    if player:getStorageValue(Storage.DjinnWar.MaridFaction.Mission03) ~= 3 then
        npcHandler:say('I\'m sorry, human. But you need Gabel\'s permission to trade with me.', cid)
        return false
    end
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
