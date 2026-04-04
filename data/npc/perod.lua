-- Perod - Converted from XML to Lua NpcType
-- Original XML: data/npc/Perod.xml
-- Original Script: data/npc/scripts/Perod.lua

local npcName = "Perod"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a perod")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(2000)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 128, lookHead = 39, lookBody = 125, lookLegs = 29, lookFeet = 114})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)


local voices = { {text = 'Stop by before embarking on your great adventure! Distance weapons and general equipment on sale today!'} }
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

npcHandler:setMessage(MESSAGE_FAREWELL, "Good bye, |PLAYERNAME|.")
npcHandler:setMessage(MESSAGE_WALKAWAY, "Good bye, |PLAYERNAME|.")
npcHandler:setCallback(CALLBACK_MESSAGE_DEFAULT, creatureSayCallback)


-- Shop items (from XML parameters)
local shopItems = {
    {id = 2456, buy = 0, sell = 100, subType = 0, name = "bow"},
    {id = 2455, buy = 0, sell = 120, subType = 0, name = "crossbow"},
    {id = 2389, buy = 0, sell = 3, subType = 0, name = "spear"},
    {id = 2544, buy = 3, sell = 0, subType = 0, name = "arrow"},
    {id = 1294, buy = 15, sell = 0, subType = 0, name = "small stone"},
    {id = 2543, buy = 4, sell = 0, subType = 0, name = "bolt"},
    {id = 18304, buy = 20, sell = 0, subType = 0, name = "crystalline arrow"},
    {id = 18435, buy = 20, sell = 0, subType = 0, name = "prismatic bolt"},
    {id = 2456, buy = 400, sell = 0, subType = 0, name = "bow"},
    {id = 3940, buy = 20, sell = 0, subType = 0, name = "camouflage backpack"},
    {id = 3939, buy = 5, sell = 0, subType = 0, name = "camouflage bag"},
    {id = 2455, buy = 500, sell = 0, subType = 0, name = "crossbow"},
    {id = 7850, buy = 5, sell = 0, subType = 0, name = "earth arrow"},
    {id = 2580, buy = 150, sell = 0, subType = 0, name = "fishing rod"},
    {id = 7840, buy = 5, sell = 0, subType = 0, name = "flaming arrow"},
    {id = 7838, buy = 5, sell = 0, subType = 0, name = "flash arrow"},
    {id = 2420, buy = 40, sell = 0, subType = 0, name = "machete"},
    {id = 7365, buy = 7, sell = 0, subType = 0, name = "onyx arrow"},
    {id = 2553, buy = 50, sell = 0, subType = 0, name = "pick"},
    {id = 7363, buy = 5, sell = 0, subType = 0, name = "piercing bolt"},
    {id = 2547, buy = 7, sell = 0, subType = 0, name = "power bolt"},
    {id = 1990, buy = 10, sell = 0, subType = 0, name = "present"},
    {id = 2120, buy = 50, sell = 0, subType = 0, name = "rope"},
    {id = 7378, buy = 15, sell = 0, subType = 0, name = "royal spear"},
    {id = 1949, buy = 5, sell = 0, subType = 0, name = "scroll"},
    {id = 2550, buy = 50, sell = 0, subType = 0, name = "scythe"},
    {id = 7839, buy = 5, sell = 0, subType = 0, name = "shiver arrow"},
    {id = 2554, buy = 50, sell = 0, subType = 0, name = "shovel"},
    {id = 7364, buy = 5, sell = 0, subType = 0, name = "sniper arrow"},
    {id = 2389, buy = 10, sell = 0, subType = 0, name = "spear"},
    {id = 2399, buy = 42, sell = 0, subType = 0, name = "throwing star"},
    {id = 2050, buy = 2, sell = 0, subType = 0, name = "torch"},
    {id = 6538, buy = 30, sell = 0, subType = 0, name = "valentine's card"},
    {id = 6091, buy = 20, sell = 0, subType = 0, name = "watch"},
    {id = 3976, buy = 1, sell = 0, subType = 0, name = "worm"},
    {id = 28413, buy = 130, sell = 0, subType = 0, name = "diamond arrow"},
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
