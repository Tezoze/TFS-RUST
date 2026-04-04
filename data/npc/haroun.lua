-- Haroun - Converted from XML to Lua NpcType
-- Original XML: data/npc/Haroun.xml
-- Original Script: data/npc/scripts/Haroun.lua

local npcName = "Haroun"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a haroun")
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
	if isInArray({"enchanted chicken wing", "boots of haste"}, msg) then
		npcHandler:say('Do you want to trade Boots of haste for Enchanted Chicken Wing?', cid)
		npcHandler.topic[cid] = 1
	elseif isInArray({"warrior sweat", "warrior helmet"}, msg) then
		npcHandler:say('Do you want to trade 4 Warrior Helmet for Warrior Sweat?', cid)
		npcHandler.topic[cid] = 2
	elseif isInArray({"fighting spirit", "royal helmet"}, msg) then
		npcHandler:say('Do you want to trade 2 Royal Helmet for Fighting Spirit', cid)
		npcHandler.topic[cid] = 3
	elseif isInArray({"magic sulphur", "fire sword"}, msg) then
		npcHandler:say('Do you want to trade 3 Fire Sword for Magic Sulphur', cid)
		npcHandler.topic[cid] = 4
	elseif isInArray({"job", "items"}, msg) then
		npcHandler:say('I trade Enchanted Chicken Wing for Boots of Haste, Warrior Sweat for 4 Warrior Helmets, Fighting Spirit for 2 Royal Helmet Magic Sulphur for 3 Fire Swords', cid)
		npcHandler.topic[cid] = 0
	elseif msgcontains(msg,'yes') and npcHandler.topic[cid] <= 4 and npcHandler.topic[cid] >= 1 then
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
	elseif msgcontains(msg,'no') and (npcHandler.topic[cid] >= 1 and npcHandler.topic[cid] <= 5) then
		npcHandler:say('Ok then.', cid)
		npcHandler.topic[cid] = 0
		npcHandler:releaseFocus(cid)
		npcHandler:resetNpc(cid)
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

npcHandler:setMessage(MESSAGE_GREET, "Be greeted, human |PLAYERNAME|. How can a humble djinn be of service?")
npcHandler:setMessage(MESSAGE_FAREWELL, "Farewell! May the serene light of the enlightened one rest shine on your travels.")
npcHandler:setMessage(MESSAGE_WALKAWAY, "Farewell, human.")
npcHandler:setMessage(MESSAGE_SENDTRADE, 'At your service, just browse through my wares.')

npcHandler:setCallback(CALLBACK_ONTRADEREQUEST, onTradeRequest)
npcHandler:setCallback(CALLBACK_MESSAGE_DEFAULT, creatureSayCallback)


-- Shop items (from XML parameters)
local shopItems = {
    {id = 2207, buy = 0, sell = 100, subType = 0, name = "melee ring"},
    {id = 2172, buy = 0, sell = 50, subType = 0, name = "bronze amulet"},
    {id = 2198, buy = 0, sell = 100, subType = 0, name = "elven amulet"},
    {id = 2199, buy = 0, sell = 50, subType = 0, name = "garlic necklace"},
    {id = 4851, buy = 0, sell = 50, subType = 0, name = "life crystal"},
    {id = 2162, buy = 0, sell = 35, subType = 0, name = "magic light wand"},
    {id = 2178, buy = 0, sell = 100, subType = 0, name = "mind stone"},
    {id = 2176, buy = 0, sell = 750, subType = 0, name = "orb"},
    {id = 2166, buy = 0, sell = 50, subType = 0, name = "power ring"},
    {id = 2165, buy = 0, sell = 200, subType = 0, name = "stealth ring"},
    {id = 2197, buy = 0, sell = 500, subType = 0, name = "stone skin amulet"},
    {id = 2189, buy = 0, sell = 2000, subType = 0, name = "wand of cosmic energy"},
    {id = 2188, buy = 0, sell = 1000, subType = 0, name = "wand of decay"},
    {id = 8921, buy = 0, sell = 1500, subType = 0, name = "wand of draconia"},
    {id = 2191, buy = 0, sell = 200, subType = 0, name = "wand of dragonbreath"},
    {id = 2187, buy = 0, sell = 3000, subType = 0, name = "wand of inferno"},
    {id = 8920, buy = 0, sell = 3600, subType = 0, name = "wand of starstorm"},
    {id = 8922, buy = 0, sell = 4400, subType = 0, name = "wand of voodoo"},
    {id = 2190, buy = 0, sell = 100, subType = 0, name = "wand of vortex"},
    {id = 2207, buy = 500, sell = 0, subType = 0, name = "melee ring"},
    {id = 2172, buy = 100, sell = 0, subType = 200, name = "bronze amulet"},
    {id = 2198, buy = 500, sell = 0, subType = 50, name = "elven amulet"},
    {id = 2162, buy = 120, sell = 0, subType = 0, name = "magic light wand"},
    {id = 2166, buy = 100, sell = 0, subType = 0, name = "power ring"},
    {id = 2165, buy = 5000, sell = 0, subType = 0, name = "stealth ring"},
    {id = 2197, buy = 5000, sell = 0, subType = 5, name = "stone skin amulet"},
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
