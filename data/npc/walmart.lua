-- Walmart - Converted from XML to Lua NpcType
-- Original XML: data/npc/Walmart.xml
-- Original Script: data/npc/scripts/Walmart.lua

local npcName = "Walmart"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a walmart")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(2000)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 128, lookHead = 58, lookBody = 68, lookLegs = 109, lookFeet = 115})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)


local function creatureSayCallback(cid, type, msg)
	if not npcHandler:isFocused(cid) then
		return false
	end

	local player = Player(cid)

	if msgcontains(msg, 'egg') then
		npcHandler:say('Do you like ten eggs for 50 gold?', cid)
			npcHandler.topic[cid] = 1
	elseif npcHandler.topic[cid] == 1 then
		if msgcontains(msg, 'yes') then
			if not player:removeMoneyNpc(50) then
				npcHandler:say('No gold, no sale, that\'s it.', cid)
				return true
			end
			npcHandler:say('Here you are.', cid)			
			player:addItem(2695, 10)
		elseif msgcontains(msg, 'no') then
			npcHandler:say('I have but a few eggs, anyway.', cid)
		end
	end
	return true
end

npcHandler:setMessage(MESSAGE_GREET, 'Welcome to the Supermarket, |PLAYERNAME|. Have a good time and eat some food!')

npcHandler:setCallback(CALLBACK_MESSAGE_DEFAULT, creatureSayCallback)
local focusModule = FocusModule:new()
focusModule:addGreetMessage('hi')
npcHandler:addModule(focusModule)


-- Shop items (from XML parameters)
local shopItems = {
    {id = 2676, buy = 2, sell = 0, subType = 0, name = "banana"},
    {id = 6574, buy = 800, sell = 0, subType = 0, name = "bar of chocolate"},
    {id = 8845, buy = 2, sell = 0, subType = 0, name = "beetroot"},
    {id = 5942, buy = 5000, sell = 0, subType = 0, name = "blessed wooden stake"},
    {id = 9674, buy = 200, sell = 0, subType = 0, name = "bottle of bug milk"},
    {id = 2691, buy = 3, sell = 0, subType = 0, name = "brown bread"},
    {id = 2789, buy = 10, sell = 0, subType = 0, name = "brown mushroom"},
    {id = 9114, buy = 3, sell = 0, subType = 0, name = "bulb of garlic"},
    {id = 6569, buy = 100, sell = 0, subType = 0, name = "candy"},
    {id = 2688, buy = 150, sell = 0, subType = 0, name = "candy cane"},
    {id = 2684, buy = 2, sell = 0, subType = 0, name = "carrot"},
    {id = 2696, buy = 5, sell = 0, subType = 0, name = "cheese"},
    {id = 6558, buy = 500, sell = 0, subType = 0, name = "concentrated demonic blood"},
    {id = 2687, buy = 2, sell = 0, subType = 0, name = "cookie"},
    {id = 2686, buy = 3, sell = 0, subType = 0, name = "corncob"},
    {id = 7966, buy = 200, sell = 0, subType = 0, name = "cream cake"},
    {id = 8842, buy = 3, sell = 0, subType = 0, name = "cucumber"},
    {id = 4298, buy = 100, sell = 0, subType = 0, name = "dead bat"},
    {id = 4265, buy = 100, sell = 0, subType = 0, name = "dead chicken"},
    {id = 2801, buy = 24, sell = 0, subType = 0, name = "fern"},
    {id = 2692, buy = 10, sell = 0, subType = 0, name = "flour"},
    {id = 6501, buy = 150, sell = 0, subType = 0, name = "gingerbreadman"},
    {id = 7159, buy = 80, sell = 0, subType = 0, name = "green perch"},
    {id = 2671, buy = 5, sell = 0, subType = 0, name = "ham"},
    {id = 5902, buy = 500, sell = 0, subType = 0, name = "honeycomb"},
    {id = 7250, buy = 100, sell = 0, subType = 0, name = "hydra tongue"},
    {id = 8844, buy = 2, sell = 0, subType = 0, name = "jalapeno pepper"},
    {id = 8841, buy = 3, sell = 0, subType = 0, name = "lemon"},
    {id = 5097, buy = 10, sell = 0, subType = 0, name = "mango"},
    {id = 2666, buy = 3, sell = 0, subType = 0, name = "meat"},
    {id = 2669, buy = 90, sell = 0, subType = 0, name = "northern pike"},
    {id = 8843, buy = 2, sell = 0, subType = 0, name = "onion"},
    {id = 2675, buy = 5, sell = 0, subType = 0, name = "orange"},
    {id = 7910, buy = 5, sell = 0, subType = 0, name = "peanut"},
    {id = 8839, buy = 3, sell = 0, subType = 0, name = "plum"},
    {id = 8838, buy = 4, sell = 0, subType = 0, name = "potato"},
    {id = 2803, buy = 200, sell = 0, subType = 0, name = "powder herb"},
    {id = 2683, buy = 10, sell = 0, subType = 0, name = "pumpkin"},
    {id = 7158, buy = 90, sell = 0, subType = 0, name = "rainbow trout"},
    {id = 2788, buy = 12, sell = 0, subType = 0, name = "red mushroom"},
    {id = 11246, buy = 500, sell = 0, subType = 0, name = "rice ball"},
    {id = 2690, buy = 2, sell = 0, subType = 0, name = "roll"},
    {id = 11373, buy = 2000, sell = 0, subType = 0, name = "sandcrawler shell"},
    {id = 2804, buy = 700, sell = 0, subType = 0, name = "shadow herb"},
    {id = 2670, buy = 45, sell = 0, subType = 0, name = "shrimp"},
    {id = 2802, buy = 200, sell = 0, subType = 0, name = "sling herb"},
    {id = 2800, buy = 21, sell = 0, subType = 0, name = "star herb"},
    {id = 2799, buy = 28, sell = 0, subType = 0, name = "stone herb"},
    {id = 2685, buy = 5, sell = 0, subType = 0, name = "tomato"},
    {id = 2805, buy = 150, sell = 0, subType = 0, name = "troll green"},
    {id = 2006, buy = 8, sell = 0, subType = 1, name = "vial of water"},
    {id = 2006, buy = 8, sell = 0, subType = 3, name = "vial of beer"},
    {id = 2006, buy = 15, sell = 0, subType = 6, name = "vial of milk"},
    {id = 2006, buy = 15, sell = 0, subType = 140, name = "vial of coconut milk"},
    {id = 2006, buy = 5, sell = 0, subType = 15, name = "vial of wine"},
    {id = 2006, buy = 5, sell = 0, subType = 43, name = "vial of mead"},
    {id = 5865, buy = 100, sell = 0, subType = 0, name = "juice squeezer"},
    {id = 2678, buy = 50, sell = 0, subType = 0, name = "coconut"},
    {id = 2787, buy = 6, sell = 0, subType = 0, name = "white mushroom"},
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

npcType:register()
