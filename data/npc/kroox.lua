-- Kroox - Converted from XML to Lua NpcType
-- Original XML: data/npc/Kroox.xml
-- Original Script: data/npc/scripts/Kroox.lua

local npcName = "Kroox"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a kroox")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(2000)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 160, lookBody = 120, lookLegs = 82, lookFeet = 95})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)


local function creatureSayCallback(cid, type, msg)
	if not npcHandler:isFocused(cid) then
		return false
	end
	local player = Player(cid)
	if msgcontains(msg, "sam sent me") or msgcontains(msg, "sam send me") then
		if player:getStorageValue(Storage.SamsOldBackpack) == 1 then
			npcHandler:say({
				"Oh, so its you, he wrote me about? Sadly I have no dwarven armor in stock. But I give you the permission to retrive one from the mines. ...",
				"The problem is, some giant spiders made the tunnels where the storage is their new home. Good luck."
			}, cid)
			player:setStorageValue(Storage.SamsOldBackpack, 2)
		end
	elseif msgcontains(msg, "measurements") then
		if player:getStorageValue(Storage.postman.Mission07) >= 1 and	player:getStorageValue(Storage.postman.MeasurementsKroox) ~= 1 then
			npcHandler:say("Hm, well I guess its ok to tell you ... <tells you about Lokurs measurements> ", cid)
			player:setStorageValue(Storage.postman.Mission07, player:getStorageValue(Storage.postman.Mission07) + 1)
			player:setStorageValue(Storage.postman.MeasurementsKroox, 1)
	else
			npcHandler:say("...", cid)
			npcHandler.topic[cid] = 0
		end
	end
	return true
end

npcHandler:setCallback(CALLBACK_MESSAGE_DEFAULT, creatureSayCallback)


-- Shop items (from XML parameters)
local shopItems = {
    {id = 2457, buy = 0, sell = 300, subType = 0, name = "steel helmet"},
    {id = 2458, buy = 0, sell = 12, subType = 0, name = "chain helmet"},
    {id = 2459, buy = 0, sell = 140, subType = 0, name = "iron helmet"},
    {id = 2460, buy = 0, sell = 30, subType = 0, name = "brass helmet"},
    {id = 2462, buy = 0, sell = 450, subType = 0, name = "devil helmet"},
    {id = 2473, buy = 0, sell = 70, subType = 0, name = "viking helmet"},
    {id = 2475, buy = 0, sell = 700, subType = 0, name = "warrior helmet"},
    {id = 2477, buy = 0, sell = 380, subType = 0, name = "knight legs"},
    {id = 2478, buy = 0, sell = 50, subType = 0, name = "brass legs"},
    {id = 2647, buy = 0, sell = 115, subType = 0, name = "plate legs"},
    {id = 2648, buy = 0, sell = 20, subType = 0, name = "chain legs"},
    {id = 2463, buy = 0, sell = 240, subType = 0, name = "plate armor"},
    {id = 2464, buy = 0, sell = 40, subType = 0, name = "chain armor"},
    {id = 2465, buy = 0, sell = 110, subType = 0, name = "brass armor"},
    {id = 2510, buy = 0, sell = 45, subType = 0, name = "plate shield"},
    {id = 2511, buy = 0, sell = 16, subType = 0, name = "brass shield"},
    {id = 2512, buy = 0, sell = 3, subType = 0, name = "wooden shield"},
    {id = 2513, buy = 0, sell = 60, subType = 0, name = "battle shield"},
    {id = 2525, buy = 0, sell = 100, subType = 0, name = "dwarven shield"},
    {id = 2457, buy = 580, sell = 0, subType = 0, name = "steel helmet"},
    {id = 2458, buy = 52, sell = 0, subType = 0, name = "chain helmet"},
    {id = 2459, buy = 390, sell = 0, subType = 0, name = "iron helmet"},
    {id = 2460, buy = 120, sell = 0, subType = 0, name = "brass helmet"},
    {id = 2478, buy = 195, sell = 0, subType = 0, name = "brass legs"},
    {id = 2648, buy = 80, sell = 0, subType = 0, name = "chain legs"},
    {id = 2463, buy = 1200, sell = 0, subType = 0, name = "plate armor"},
    {id = 2464, buy = 200, sell = 0, subType = 0, name = "chain armor"},
    {id = 2465, buy = 450, sell = 0, subType = 0, name = "brass armor"},
    {id = 2509, buy = 240, sell = 0, subType = 0, name = "steel shield"},
    {id = 2510, buy = 125, sell = 0, subType = 0, name = "plate shield"},
    {id = 2525, buy = 500, sell = 0, subType = 0, name = "dwarven shield"},
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
