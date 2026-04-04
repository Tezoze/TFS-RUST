-- Serafin - Converted from XML to Lua NpcType
-- Original XML: data/npc/Serafin.xml
-- Original Script: data/npc/scripts/Serafin.lua

local npcName = "Serafin"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a serafin")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(2000)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 129, lookHead = 96, lookBody = 123, lookLegs = 86, lookFeet = 98})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)


local function creatureSayCallback(cid, type, msg)
	if not npcHandler:isFocused(cid) then
		return false
	end

	local player = Player(cid)
	
	-- Blood Brothers Quest - Garlic Cookie test
	if msgcontains(msg, "garlic cookie") or msgcontains(msg, "cookie") then
		if player:getStorageValue(Storage.BloodBrothers.Mission02) == 1 then
			if player:getItemCount(9116) > 0 then -- Garlic Cookie item ID
				player:removeItem(9116, 1)
				local currentCount = player:getStorageValue(Storage.BloodBrothers.GarlicCookieCount)
				if currentCount == -1 then currentCount = 0 end
				player:setStorageValue(Storage.BloodBrothers.GarlicCookieCount, currentCount + 1)
				player:setStorageValue(Storage.BloodBrothers.SerafinSuspect, 1)
				npcHandler:say("A cookie? That's very kind of you! *takes a bite* Mmm, delicious! Thank you!", cid)
			else
				npcHandler:say("I'd love a cookie, but I don't see one!", cid)
			end
		else
			npcHandler:say("That's very kind of you, but I'm not particularly hungry right now.", cid)
		end
	elseif msgcontains(msg, "blood crystal") then
		npcHandler:say("This is a fruit store. No slaughterhouse.", cid)
		return true
	end

	return true
end

npcHandler:setCallback(CALLBACK_MESSAGE_DEFAULT, creatureSayCallback)


-- Shop items (from XML parameters)
local shopItems = {
    {id = 2675, buy = 10, sell = 0, subType = 0, name = "orange"},
    {id = 2676, buy = 5, sell = 0, subType = 0, name = "banana"},
    {id = 2679, buy = 1, sell = 0, subType = 0, name = "cherry"},
    {id = 2680, buy = 2, sell = 0, subType = 0, name = "strawberry"},
    {id = 2681, buy = 3, sell = 0, subType = 0, name = "grapes"},
    {id = 2682, buy = 10, sell = 0, subType = 0, name = "melon"},
    {id = 2683, buy = 10, sell = 0, subType = 0, name = "pumpkin"},
    {id = 2684, buy = 3, sell = 0, subType = 0, name = "carrot"},
    {id = 2686, buy = 3, sell = 0, subType = 0, name = "corncob"},
    {id = 2787, buy = 10, sell = 0, subType = 0, name = "white mushroom"},
    {id = 5097, buy = 10, sell = 0, subType = 0, name = "mango"},
    {id = 8838, buy = 4, sell = 0, subType = 0, name = "potato"},
    {id = 8841, buy = 3, sell = 0, subType = 0, name = "lemon"},
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

npcHandler:addModule(FocusModule:new())
npcType:register()
