-- Aldo - Converted from XML to Lua NpcType
-- Original XML: data/npc/Aldo.xml
-- Original Script: data/npc/scripts/Aldo.lua

local npcName = "Aldo"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a aldo")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(2000)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 128, lookHead = 40, lookBody = 37, lookLegs = 116, lookFeet = 95})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)


local function creatureSayCallback(cid, type, msg)
	if not npcHandler:isFocused(cid) then
		return false
	end
	local player = Player(cid)
	if isInArray({"soft boots", "repair", "soft", "boots"}, msg) then
		npcHandler:say("Do you want to repair your worn soft boots for 10000 gold coins?", cid)
		npcHandler.topic[cid] = 1
	elseif msgcontains(msg, 'yes') and npcHandler.topic[cid] == 1 then
		npcHandler.topic[cid] = 0
		if player:getItemCount(10021) == 0 then
			npcHandler:say("Sorry, you don't have the item.", cid)
			return true
		end

		if not player:removeMoneyNpc(10000) then
			npcHandler:say("Sorry, you don't have enough gold.", cid)
			return true
		end

		player:removeItem(10021, 1)
		player:addItem(6132, 1)
		npcHandler:say("Here you are.", cid)
	elseif msgcontains(msg, 'no') and npcHandler.topic[cid] == 1 then
		npcHandler.topic[cid] = 0
		npcHandler:say("Ok then.", cid)


	end

	return true
end

npcHandler:setCallback(CALLBACK_MESSAGE_DEFAULT, creatureSayCallback)


-- Shop items (from XML parameters)
local shopItems = {
    {id = 2460, buy = 0, sell = 30, subType = 0, name = "Brass Helmet"},
    {id = 2478, buy = 0, sell = 49, subType = 0, name = "Brass Legs"},
    {id = 2458, buy = 0, sell = 17, subType = 0, name = "Chain Helmet"},
    {id = 2648, buy = 0, sell = 25, subType = 0, name = "Chain Legs"},
    {id = 2459, buy = 0, sell = 150, subType = 0, name = "Iron Helmet"},
    {id = 2643, buy = 0, sell = 2, subType = 0, name = "Leather Boots"},
    {id = 2461, buy = 0, sell = 4, subType = 0, name = "Leather Helmet"},
    {id = 2649, buy = 0, sell = 9, subType = 0, name = "Leather Legs"},
    {id = 2480, buy = 0, sell = 22, subType = 0, name = "Legion Helmet"},
    {id = 2647, buy = 0, sell = 115, subType = 0, name = "Plate Legs"},
    {id = 2559, buy = 0, sell = 5, subType = 0, name = "Small Axe"},
    {id = 2481, buy = 0, sell = 16, subType = 0, name = "Soldier Helmet"},
    {id = 2457, buy = 0, sell = 293, subType = 0, name = "Steel Helmet"},
    {id = 2482, buy = 0, sell = 20, subType = 0, name = "Studded Helmet"},
    {id = 2468, buy = 0, sell = 15, subType = 0, name = "Studded Legs"},
    {id = 2473, buy = 0, sell = 66, subType = 0, name = "Viking Helmet"},
    {id = 2460, buy = 120, sell = 0, subType = 0, name = "Brass Helmet"},
    {id = 2478, buy = 195, sell = 0, subType = 0, name = "Brass Legs"},
    {id = 2458, buy = 52, sell = 0, subType = 0, name = "Chain Helmet"},
    {id = 2648, buy = 80, sell = 0, subType = 0, name = "Chain Legs"},
    {id = 2459, buy = 390, sell = 0, subType = 0, name = "Iron Helmet"},
    {id = 2643, buy = 10, sell = 0, subType = 0, name = "Leather Boots"},
    {id = 2461, buy = 12, sell = 0, subType = 0, name = "Leather Helmet"},
    {id = 2649, buy = 10, sell = 0, subType = 0, name = "Leather Legs"},
    {id = 2642, buy = 2, sell = 0, subType = 0, name = "Sandals"},
    {id = 2481, buy = 110, sell = 0, subType = 0, name = "Soldier Helmet"},
    {id = 2457, buy = 580, sell = 0, subType = 0, name = "Steel Helmet"},
    {id = 2482, buy = 63, sell = 0, subType = 0, name = "Studded Helmet"},
    {id = 2468, buy = 50, sell = 0, subType = 0, name = "Studded Legs"},
    {id = 2473, buy = 265, sell = 0, subType = 0, name = "Viking Helmet"},
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
