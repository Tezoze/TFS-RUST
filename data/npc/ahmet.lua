-- Ahmet - Converted from XML to Lua NpcType
-- Original XML: data/npc/Ahmet.xml
-- Original Script: data/npc/scripts/Ahmet.lua

local npcName = "Ahmet"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a ahmet")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(2000)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 130, lookHead = 57, lookBody = 116, lookLegs = 97, lookFeet = 114})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)


local function creatureSayCallback(cid, type, msg)
	if not npcHandler:isFocused(cid) then
		return false
	end
	local player = Player(cid)
	if msgcontains(msg, 'documents') then
		if player:getStorageValue(Storage.thievesGuild.Mission04) == 2 then
			player:setStorageValue(Storage.thievesGuild.Mission04, 3)
			npcHandler:say({
				'You need some forged documents? But I will only forge something for a friend. ...',
				'The nomads at the northern oasis killed someone dear to me. Go and kill at least one of them, then we talk about your document.'
			}, cid)
		elseif player:getStorageValue(Storage.thievesGuild.Mission04) == 4 then
			npcHandler:say('The slayer of my enemies is my friend! For a mere 1000 gold I will create the documents you need. Are you interested?', cid)
			npcHandler.topic[cid] = 1
		end
	elseif msgcontains(msg, 'mission') or msgcontains(msg, 'quest') then
		if player:getStorageValue(Storage.QuestChests.StealFromThieves) < 1 then
			npcHandler:say({
				"What are you talking about?? I was robbed!!!! Someone catch those filthy thieves!!!!! GUARDS! ...",
				"<nothing happens>....<SIGH> Like usual, they hide at the slightest sign of trouble! YOU! Want to earn some quick money?"
			}, cid)
			npcHandler.topic[cid] = 2
		elseif player:getStorageValue(Storage.QuestChests.StealFromThieves) == 1 or player:getStorageValue(Storage.QuestChests.StealFromThieves) == 2 then
			npcHandler:say('Did you find my stuff?', cid)
			npcHandler.topic[cid] = 3
		end
	elseif msgcontains(msg, 'yes') then
		if npcHandler.topic[cid] == 1 then
			if player:removeMoneyNpc(1000) then
				player:addItem(8694, 1)
				player:setStorageValue(Storage.thievesGuild.Mission04, 5)
				npcHandler:say('And here they are! Now forget where you got them from.', cid)
			else
				npcHandler:say('You don\'t have enough money.', cid)
			end
			npcHandler.topic[cid] = 0
		elseif npcHandler.topic[cid] == 2 then
			npcHandler:say({
				"Of course you do! Go hunt down the thieves and bring back the stuff they have stolen from me. ...",
				" I saw them running out of town and then to the north. Maybe they hide at the oasis."
			}, cid)
			npcHandler.topic[cid] = 0
			player:setStorageValue(Storage.QuestChests.StealFromThieves, 1)
		elseif npcHandler.topic[cid] == 3 then
			if player:removeItem(7587, 1) then
				npcHandler:say('GREAT! If you ever need a job as my personal security guard, let me know. Here is the reward I promised you.', cid)
				player:setStorageValue(Storage.QuestChests.StealFromThieves, 3)
				player:addItem(2148, 100)
				player:addItem(2789, 100)
				npcHandler.topic[cid] = 0
			else
				npcHandler:say('Come back when you find my stuff.', cid)
				npcHandler.topic[cid] = 0
			end
		end
	end
	return true
end

npcHandler:setCallback(CALLBACK_MESSAGE_DEFAULT, creatureSayCallback)


-- Shop items (from XML parameters)
local shopItems = {
    {id = 2578, buy = 0, sell = 75, subType = 0, name = "closed trap"},
    {id = 2416, buy = 0, sell = 50, subType = 0, name = "crowbar"},
    {id = 2580, buy = 0, sell = 40, subType = 0, name = "fishing rod"},
    {id = 2420, buy = 0, sell = 6, subType = 0, name = "machete"},
    {id = 2553, buy = 0, sell = 15, subType = 0, name = "pick"},
    {id = 2120, buy = 0, sell = 15, subType = 0, name = "rope"},
    {id = 2550, buy = 0, sell = 10, subType = 0, name = "scythe"},
    {id = 2554, buy = 0, sell = 8, subType = 0, name = "shovel"},
    {id = 2036, buy = 0, sell = 6, subType = 0, name = "watch"},
    {id = 2556, buy = 0, sell = 15, subType = 0, name = "wooden hammer"},
    {id = 1989, buy = 6, sell = 0, subType = 0, name = "basket"},
    {id = 2007, buy = 3, sell = 0, subType = 0, name = "bottle"},
    {id = 2005, buy = 4, sell = 0, subType = 0, name = "bucket"},
    {id = 2042, buy = 8, sell = 0, subType = 0, name = "candelabrum"},
    {id = 2047, buy = 2, sell = 0, subType = 0, name = "candlestick"},
    {id = 2578, buy = 280, sell = 0, subType = 0, name = "closed trap"},
    {id = 2416, buy = 260, sell = 0, subType = 0, name = "crowbar"},
    {id = 2013, buy = 2, sell = 0, subType = 0, name = "cup"},
    {id = 1952, buy = 12, sell = 0, subType = 0, name = "document"},
    {id = 2580, buy = 40, sell = 0, subType = 0, name = "fishing rod"},
    {id = 2004, buy = 10, sell = 0, subType = 0, name = "golden backpack"},
    {id = 1997, buy = 4, sell = 0, subType = 0, name = "golden bag"},
    {id = 2420, buy = 6, sell = 0, subType = 0, name = "machete"},
    {id = 1951, buy = 8, sell = 0, subType = 0, name = "parchment"},
    {id = 2553, buy = 50, sell = 0, subType = 0, name = "pick"},
    {id = 2035, buy = 6, sell = 0, subType = 0, name = "plate"},
    {id = 1990, buy = 10, sell = 0, subType = 0, name = "present"},
    {id = 2120, buy = 50, sell = 0, subType = 0, name = "rope"},
    {id = 1949, buy = 5, sell = 0, subType = 0, name = "scroll"},
    {id = 2550, buy = 50, sell = 0, subType = 0, name = "scythe"},
    {id = 2554, buy = 10, sell = 0, subType = 0, name = "shovel"},
    {id = 2050, buy = 2, sell = 0, subType = 0, name = "torch"},
    {id = 2006, buy = 8, sell = 0, subType = 0, name = "vial of water"},
    {id = 2036, buy = 20, sell = 0, subType = 0, name = "watch"},
    {id = 3976, buy = 1, sell = 0, subType = 0, name = "worm"},
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
