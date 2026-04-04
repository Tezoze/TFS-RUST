-- Pagazin - Converted from XML to Lua NpcType
-- Original XML: data/npc/Pagazin.xml
-- Original Script: data/npc/scripts/Pagazin.lua

local npcName = "Pagazin"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a pagazin")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(2000)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 22})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)


local function creatureSayCallback(cid, type, msg)
	if not npcHandler:isFocused(cid) then
		return false
	end
	
	return true
end

npcHandler:setMessage(MESSAGE_GREET, 'Welcome to the Creature Products Supermarket, |PLAYERNAME|. Have a good time!')
npcHandler:setCallback(CALLBACK_MESSAGE_DEFAULT, creatureSayCallback)
local focusModule = FocusModule:new()
focusModule:addGreetMessage('hi')
npcHandler:addModule(focusModule)


-- Shop items (from XML parameters)
local shopItems = {
    {id = 8309, buy = 0, sell = 10, subType = 0, name = "nail"},
    {id = 5894, buy = 0, sell = 50, subType = 0, name = "bat wing"},
    {id = 5898, buy = 0, sell = 80, subType = 0, name = "bonelord eye"},
    {id = 5878, buy = 0, sell = 80, subType = 0, name = "minotaur leather"},
    {id = 5922, buy = 0, sell = 90, subType = 0, name = "holy orchid"},
    {id = 2193, buy = 0, sell = 100, subType = 0, name = "ankh"},
    {id = 6098, buy = 0, sell = 100, subType = 0, name = "eye patch"},
    {id = 5877, buy = 0, sell = 100, subType = 0, name = "green dragon leather"},
    {id = 5920, buy = 0, sell = 100, subType = 0, name = "green dragon scale"},
    {id = 6097, buy = 0, sell = 100, subType = 0, name = "hook"},
    {id = 6126, buy = 0, sell = 100, subType = 0, name = "peg leg"},
    {id = 3956, buy = 0, sell = 100, subType = 0, name = "tusk"},
    {id = 5899, buy = 0, sell = 90, subType = 0, name = "turtle shell"},
    {id = 5896, buy = 0, sell = 100, subType = 0, name = "bear paw"},
    {id = 2159, buy = 0, sell = 100, subType = 0, name = "scarab coin"},
    {id = 5890, buy = 0, sell = 30, subType = 0, name = "chicken feather"},
    {id = 5902, buy = 0, sell = 40, subType = 0, name = "honeycomb"},
    {id = 2743, buy = 0, sell = 50, subType = 0, name = "heaven blossom"},
    {id = 5881, buy = 0, sell = 120, subType = 0, name = "lizard scale"},
    {id = 5876, buy = 0, sell = 150, subType = 0, name = "lizard leather"},
    {id = 11113, buy = 0, sell = 150, subType = 0, name = "orc tooth"},
    {id = 5925, buy = 0, sell = 70, subType = 0, name = "hardened bone"},
    {id = 5897, buy = 0, sell = 70, subType = 0, name = "wolf paw"},
    {id = 5883, buy = 0, sell = 120, subType = 0, name = "ape fur"},
    {id = 5912, buy = 0, sell = 200, subType = 0, name = "blue piece of cloth"},
    {id = 5913, buy = 0, sell = 100, subType = 0, name = "brown piece of cloth"},
    {id = 5910, buy = 0, sell = 200, subType = 0, name = "green piece of cloth"},
    {id = 5948, buy = 0, sell = 200, subType = 0, name = "red dragon leather"},
    {id = 5882, buy = 0, sell = 200, subType = 0, name = "red dragon scale"},
    {id = 5893, buy = 0, sell = 250, subType = 0, name = "perfect behemoth fang"},
    {id = 5911, buy = 0, sell = 300, subType = 0, name = "red piece of cloth"},
    {id = 5909, buy = 0, sell = 100, subType = 0, name = "white piece of cloth"},
    {id = 5914, buy = 0, sell = 150, subType = 0, name = "yellow piece of cloth"},
    {id = 5905, buy = 0, sell = 400, subType = 0, name = "vampire dust"},
    {id = 5880, buy = 0, sell = 500, subType = 0, name = "iron ore"},
    {id = 5888, buy = 0, sell = 500, subType = 0, name = "piece of hell steel"},
    {id = 6500, buy = 0, sell = 1000, subType = 0, name = "demonic essence"},
    {id = 5895, buy = 0, sell = 150, subType = 0, name = "fish fin"},
    {id = 5879, buy = 0, sell = 100, subType = 0, name = "spider silk"},
    {id = 5527, buy = 0, sell = 300, subType = 0, name = "demon dust"},
    {id = 5930, buy = 0, sell = 2000, subType = 0, name = "behemoth claw"},
    {id = 7290, buy = 0, sell = 2000, subType = 0, name = "shard"},
    {id = 5889, buy = 0, sell = 3000, subType = 0, name = "piece of draconian steel"},
    {id = 9020, buy = 0, sell = 1000, subType = 0, name = "vampire lord token"},
    {id = 5887, buy = 0, sell = 10000, subType = 0, name = "piece of royal steel"},
    {id = 5892, buy = 0, sell = 15000, subType = 0, name = "huge chunk of crude iron"},
    {id = 5919, buy = 0, sell = 100000, subType = 0, name = "dragon claw"},
    {id = 8309, buy = 30, sell = 0, subType = 0, name = "nail"},
    {id = 5894, buy = 200, sell = 0, subType = 0, name = "bat wing"},
    {id = 5898, buy = 200, sell = 0, subType = 0, name = "bonelord eye"},
    {id = 5878, buy = 200, sell = 0, subType = 0, name = "minotaur leather"},
    {id = 5922, buy = 225, sell = 0, subType = 0, name = "holy orchid"},
    {id = 2193, buy = 250, sell = 0, subType = 0, name = "ankh"},
    {id = 6098, buy = 250, sell = 0, subType = 0, name = "eye patch"},
    {id = 5877, buy = 250, sell = 0, subType = 0, name = "green dragon leather"},
    {id = 5920, buy = 250, sell = 0, subType = 0, name = "green dragon scale"},
    {id = 6097, buy = 250, sell = 0, subType = 0, name = "hook"},
    {id = 6126, buy = 250, sell = 0, subType = 0, name = "peg leg"},
    {id = 3956, buy = 250, sell = 0, subType = 0, name = "tusk"},
    {id = 5899, buy = 300, sell = 0, subType = 0, name = "turtle shell"},
    {id = 5896, buy = 300, sell = 0, subType = 0, name = "bear paw"},
    {id = 2159, buy = 300, sell = 0, subType = 0, name = "scarab coin"},
    {id = 5890, buy = 300, sell = 0, subType = 0, name = "chicken feather"},
    {id = 5902, buy = 300, sell = 0, subType = 0, name = "honeycomb"},
    {id = 2743, buy = 300, sell = 0, subType = 0, name = "heaven blossom"},
    {id = 5881, buy = 300, sell = 0, subType = 0, name = "lizard scale"},
    {id = 5876, buy = 375, sell = 0, subType = 0, name = "lizard leather"},
    {id = 11113, buy = 375, sell = 0, subType = 0, name = "orc tooth"},
    {id = 5925, buy = 400, sell = 0, subType = 0, name = "hardened bone"},
    {id = 5897, buy = 400, sell = 0, subType = 0, name = "wolf paw"},
    {id = 5883, buy = 400, sell = 0, subType = 0, name = "ape fur"},
    {id = 5912, buy = 500, sell = 0, subType = 0, name = "blue piece of cloth"},
    {id = 5913, buy = 500, sell = 0, subType = 0, name = "brown piece of cloth"},
    {id = 5910, buy = 500, sell = 0, subType = 0, name = "green piece of cloth"},
    {id = 5948, buy = 500, sell = 0, subType = 0, name = "red dragon leather"},
    {id = 5882, buy = 500, sell = 0, subType = 0, name = "red dragon scale"},
    {id = 5893, buy = 625, sell = 0, subType = 0, name = "perfect behemoth fang"},
    {id = 5911, buy = 750, sell = 0, subType = 0, name = "red piece of cloth"},
    {id = 5909, buy = 750, sell = 0, subType = 0, name = "white piece of cloth"},
    {id = 5914, buy = 750, sell = 0, subType = 0, name = "yellow piece of cloth"},
    {id = 5905, buy = 800, sell = 0, subType = 0, name = "vampire dust"},
    {id = 5880, buy = 1250, sell = 0, subType = 0, name = "iron ore"},
    {id = 5888, buy = 1250, sell = 0, subType = 0, name = "piece of hell steel"},
    {id = 6500, buy = 2500, sell = 0, subType = 0, name = "demonic essence"},
    {id = 5895, buy = 3000, sell = 0, subType = 0, name = "fish fin"},
    {id = 5879, buy = 3000, sell = 0, subType = 0, name = "spider silk"},
    {id = 5527, buy = 4000, sell = 0, subType = 0, name = "demon dust"},
    {id = 5930, buy = 5000, sell = 0, subType = 0, name = "behemoth claw"},
    {id = 7290, buy = 5000, sell = 0, subType = 0, name = "shard"},
    {id = 5889, buy = 7500, sell = 0, subType = 0, name = "piece of draconian steel"},
    {id = 9020, buy = 8000, sell = 0, subType = 0, name = "vampire lord token"},
    {id = 5887, buy = 25000, sell = 0, subType = 0, name = "piece of royal steel"},
    {id = 5892, buy = 37500, sell = 0, subType = 0, name = "huge chunk of crude iron"},
    {id = 5919, buy = 500000, sell = 0, subType = 0, name = "dragon claw"},
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

npcType:register()
