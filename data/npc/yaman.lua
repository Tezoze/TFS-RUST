-- Yaman - Converted from XML to Lua NpcType
-- Original XML: data/npc/Yaman.xml
-- Original Script: data/npc/scripts/Yaman.lua

local npcName = "Yaman"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a yaman")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(2000)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 103})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)


local function creatureSayCallback(cid, type, msg)
	if not npcHandler:isFocused(cid) then
		return false
	end

	local player = Player(cid)

	if isInArray({"enchanted chicken wing", "boots of haste", "Enchanted Chicken Wing", "Boots of Haste"}, msg) then
		npcHandler:say('Do you want to trade Boots of haste for Enchanted Chicken Wing?', cid)
		npcHandler.topic[cid] = 1
	elseif isInArray({"warrior sweat", "warrior helmet", "Warrior Sweat", "Warrior Helmet"}, msg) then
		npcHandler:say('Do you want to trade 4 Warrior Helmet for Warrior Sweat?', cid)
		npcHandler.topic[cid] = 2
	elseif isInArray({"fighting spirit", "royal helmet", "Fighting Spirit", "Royal Helmet"}, msg) then
		npcHandler:say('Do you want to trade 2 Royal Helmet for Fighting Spirit', cid)
		npcHandler.topic[cid] = 3
	elseif isInArray({"magic sulphur", "fire sword", "Magic Sulphur", "Fire Sword"}, msg) then
		npcHandler:say('Do you want to trade 3 Fire Sword for Magic Sulphur', cid)
		npcHandler.topic[cid] = 4
	elseif isInArray({"job", "items", "Items", "Job"}, msg) then
		npcHandler:say('I trade Enchanted Chicken Wing for Boots of Haste, Warrior Sweat for 4 Warrior Helmets, Fighting Spirit for 2 Royal Helmet Magic Sulphur for 3 Fire Swords', cid)
		npcHandler.topic[cid] = 0
	elseif msgcontains(msg, 'cookie') then
		if player:getStorageValue(Storage.WhatAFoolishQuest.Questline) == 31
				and player:getStorageValue(Storage.WhatAFoolishQuest.CookieDelivery.Djinn) ~= 1 then
			npcHandler:say('You brought cookies! How nice of you! Can I have one?', cid)
			npcHandler.topic[cid] = 5
		end
	elseif msgcontains(msg,'yes') then
		if npcHandler.topic[cid] >= 1 and npcHandler.topic[cid] <= 4 then
			local trade = {
					{ NeedItem = 2195, Ncount = 1, GiveItem = 5891, Gcount = 1}, -- Enchanted Chicken Wing
					{ NeedItem = 2475, Ncount = 4, GiveItem = 5885, Gcount = 1}, -- Flask of Warrior's Sweat
					{ NeedItem = 2498, Ncount = 2, GiveItem = 5884, Gcount = 1}, -- Spirit Container
					{ NeedItem = 2392, Ncount = 3, GiveItem = 5904, Gcount = 1}  -- Magic Sulphur
			}

			if player:getItemCount(trade[npcHandler.topic[cid]].NeedItem) >= trade[npcHandler.topic[cid]].Ncount then
				player:removeItem(trade[npcHandler.topic[cid]].NeedItem, trade[npcHandler.topic[cid]].Ncount)
				player:addItem(trade[npcHandler.topic[cid]].GiveItem, trade[npcHandler.topic[cid]].Gcount)
				return npcHandler:say('Here you are.', cid)
			else
				npcHandler:say('Sorry but you don\'t have the item.', cid)
			end
		elseif npcHandler.topic[cid] == 5 then
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
	elseif msgcontains(msg,'no') then
		if npcHandler.topic[cid] >= 1 and npcHandler.topic[cid] <= 4 then
			npcHandler:say('Ok then.', cid)
			npcHandler.topic[cid] = 0
		elseif npcHandler.topic[cid] == 5 then
			npcHandler:say('I see.', cid)
			npcHandler.topic[cid] = 0
		end
	end
	return true
end

local function onTradeRequest(cid)
	local player = Player(cid)
	
	if player:getStorageValue(Storage.DjinnWar.EfreetFaction.Mission03) ~= 3 then
		npcHandler:say('I\'m sorry, but you don\'t have Malor\'s permission to trade with me.', cid)
		return false
	end

	return true
end

npcHandler:setMessage(MESSAGE_GREET, "Be greeted, human |PLAYERNAME|. How can a humble djinn be of service?")
npcHandler:setMessage(MESSAGE_FAREWELL, "Farewell, human.")
npcHandler:setMessage(MESSAGE_WALKAWAY, "Farewell, human.")
npcHandler:setMessage(MESSAGE_SENDTRADE, 'At your service, just browse through my wares.')

npcHandler:setCallback(CALLBACK_ONTRADEREQUEST, onTradeRequest)
npcHandler:setCallback(CALLBACK_MESSAGE_DEFAULT, creatureSayCallback)

local focusModule = FocusModule:new()
focusModule:addGreetMessage('hi')
focusModule:addGreetMessage('hello')
focusModule:addGreetMessage('djanni\'hah')
npcHandler:addModule(focusModule)


-- Shop items (from XML parameters)
local shopItems = {
    {id = 2327, buy = 0, sell = 100, subType = 0, name = "ankh"},
    {id = 2201, buy = 0, sell = 100, subType = 0, name = "dragon necklace"},
    {id = 2213, buy = 0, sell = 100, subType = 0, name = "dwarven ring"},
    {id = 2167, buy = 0, sell = 100, subType = 0, name = "energy ring"},
    {id = 2183, buy = 0, sell = 3000, subType = 0, name = "hailstorm rod"},
    {id = 2168, buy = 0, sell = 50, subType = 0, name = "life ring"},
    {id = 2164, buy = 0, sell = 250, subType = 0, name = "might ring"},
    {id = 2186, buy = 0, sell = 200, subType = 0, name = "moonlight rod"},
    {id = 2194, buy = 0, sell = 50, subType = 0, name = "mysterious fetish"},
    {id = 2185, buy = 0, sell = 1000, subType = 0, name = "necrotic rod"},
    {id = 8911, buy = 0, sell = 1500, subType = 0, name = "northwind rod"},
    {id = 2200, buy = 0, sell = 100, subType = 0, name = "protection amulet"},
    {id = 2216, buy = 0, sell = 100, subType = 0, name = "ring of healing"},
    {id = 2170, buy = 0, sell = 50, subType = 0, name = "silver amulet"},
    {id = 2182, buy = 0, sell = 100, subType = 0, name = "snakebite rod"},
    {id = 8912, buy = 0, sell = 3600, subType = 0, name = "springsprout rod"},
    {id = 2161, buy = 0, sell = 30, subType = 0, name = "strange talisman"},
    {id = 2181, buy = 0, sell = 2000, subType = 0, name = "terra rod"},
    {id = 2169, buy = 0, sell = 100, subType = 0, name = "time ring"},
    {id = 8910, buy = 0, sell = 4400, subType = 0, name = "underworld rod"},
    {id = 2201, buy = 1000, sell = 0, subType = 200, name = "dragon necklace"},
    {id = 2213, buy = 2000, sell = 0, subType = 0, name = "dwarven ring"},
    {id = 2167, buy = 2000, sell = 0, subType = 0, name = "energy ring"},
    {id = 2168, buy = 900, sell = 0, subType = 0, name = "life ring"},
    {id = 2164, buy = 5000, sell = 0, subType = 20, name = "might ring"},
    {id = 2200, buy = 700, sell = 0, subType = 250, name = "protection amulet"},
    {id = 2216, buy = 2000, sell = 0, subType = 0, name = "ring of healing"},
    {id = 2170, buy = 100, sell = 0, subType = 200, name = "silver amulet"},
    {id = 2161, buy = 100, sell = 0, subType = 200, name = "strange talisman"},
    {id = 2169, buy = 2000, sell = 0, subType = 0, name = "time ring"},
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
    -- Quest check: must have Malor's permission (Efreet Faction Mission03 = 3)
    if player:getStorageValue(Storage.DjinnWar.EfreetFaction.Mission03) ~= 3 then
        npcHandler:say('I\'m sorry, but you don\'t have Malor\'s permission to trade with me.', cid)
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

npcType:register()
