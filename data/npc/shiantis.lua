-- Shiantis - Converted from XML to Lua NpcType
-- Original XML: data/npc/Shiantis.xml
-- Original Script: data/npc/scripts/Shiantis.lua

local npcName = "Shiantis"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a shiantis")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(2000)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 136, lookBody = 36, lookLegs = 13, lookFeet = 76})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)


local voices = { {text = 'Containers, decoration and general goods, all here!'} }
npcHandler:addModule(VoiceModule:new(voices))

local function creatureSayCallback(cid, type, msg)
	if not npcHandler:isFocused(cid) then
		return false
	end
	if msgcontains(msg, "football") then
		npcHandler:say("Do you want to buy a football for 111 gold?", cid)
		npcHandler.topic[cid] = 1
	elseif msgcontains(msg, "yes") then
		if npcHandler.topic[cid] == 1 then
			local player = Player(cid)
			if player:getMoney() + player:getBankBalance() >= 111 then
				npcHandler:say("Here it is.", cid)
				player:addItem(2109, 1)
				player:removeMoneyNpc(111)
			else
				npcHandler:say("You don't have enough money.", cid)
			end
			npcHandler.topic[cid] = 0
		end
	end
	return true
end

npcHandler:setMessage(MESSAGE_GREET, "Oh, please come in, |PLAYERNAME|. What can I do for you? If you need adventure equipment, ask me for a {trade}.")
npcHandler:setMessage(MESSAGE_FAREWELL, "Good bye, |PLAYERNAME|.")
npcHandler:setMessage(MESSAGE_WALKAWAY, "Good bye, |PLAYERNAME|.")
npcHandler:setMessage(MESSAGE_SENDTRADE, "Of course, just browse through my wares.")
npcHandler:setCallback(CALLBACK_MESSAGE_DEFAULT, creatureSayCallback)


-- Shop items (from XML parameters)
local shopItems = {
    {id = 2006, buy = 0, sell = 5, subType = 0, name = "vial"},
    {id = 3922, buy = 50, sell = 0, subType = 0, name = "birdcage kit"},
    {id = 1971, buy = 15, sell = 0, subType = 0, name = "book"},
    {id = 1972, buy = 15, sell = 0, subType = 0, name = "book"},
    {id = 1973, buy = 15, sell = 0, subType = 0, name = "book"},
    {id = 2041, buy = 8, sell = 0, subType = 0, name = "candelabrum"},
    {id = 2047, buy = 2, sell = 0, subType = 0, name = "candlestick"},
    {id = 8692, buy = 200, sell = 0, subType = 0, name = "chimney kit"},
    {id = 3932, buy = 25, sell = 0, subType = 0, name = "coal basin kit"},
    {id = 1877, buy = 40, sell = 0, subType = 0, name = "cuckoo clock"},
    {id = 1968, buy = 12, sell = 0, subType = 0, name = "document"},
    {id = 5928, buy = 50, sell = 0, subType = 0, name = "goldfish bowl"},
    {id = 3923, buy = 50, sell = 0, subType = 0, name = "globe kit"},
    {id = 6372, buy = 80, sell = 0, subType = 0, name = "oven kit"},
    {id = 1969, buy = 8, sell = 0, subType = 0, name = "parchment"},
    {id = 3927, buy = 75, sell = 0, subType = 0, name = "pendulum clock kit"},
    {id = 1852, buy = 50, sell = 0, subType = 0, name = "picture"},
    {id = 1853, buy = 50, sell = 0, subType = 0, name = "picture"},
    {id = 1854, buy = 50, sell = 0, subType = 0, name = "picture"},
    {id = 1990, buy = 10, sell = 0, subType = 0, name = "present"},
    {id = 2000, buy = 20, sell = 0, subType = 0, name = "red backpack"},
    {id = 1993, buy = 5, sell = 0, subType = 0, name = "red bag"},
    {id = 3926, buy = 30, sell = 0, subType = 0, name = "rocking horse kit"},
    {id = 1949, buy = 5, sell = 0, subType = 0, name = "scroll"},
    {id = 3924, buy = 35, sell = 0, subType = 0, name = "table lamp kit"},
    {id = 3925, buy = 70, sell = 0, subType = 0, name = "telescope kit"},
    {id = 2050, buy = 2, sell = 0, subType = 0, name = "torch"},
    {id = 2006, buy = 20, sell = 0, subType = 11, name = "vial of oil"},
    {id = 6091, buy = 20, sell = 0, subType = 0, name = "watch"},
    {id = 1845, buy = 40, sell = 0, subType = 0, name = "wall mirror"},
    {id = 1848, buy = 40, sell = 0, subType = 0, name = "wall mirror"},
    {id = 1851, buy = 40, sell = 0, subType = 0, name = "wall mirror"},
    {id = 2093, buy = 40, sell = 0, subType = 0, name = "water pipe"},
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
