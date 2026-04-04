-- Maryza - Converted from XML to Lua NpcType
-- Original XML: data/npc/Maryza.xml
-- Original Script: data/npc/scripts/Maryza.lua

local npcName = "Maryza"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a maryza")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(2000)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 160, lookHead = 41, lookBody = 51, lookLegs = 70, lookFeet = 95})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)


local function creatureSayCallback(cid, type, msg)
	if not npcHandler:isFocused(cid) then
		return false
	end

	local player = Player(cid)

	if msgcontains(msg, 'cookbook') then
		if player:getStorageValue(Storage.MaryzaCookbook) ~= 1 then
			npcHandler:say('The cookbook of the famous dwarven kitchen. You\'re lucky. I have a few copies on sale. Do you like one for 150 gold?', cid)
			npcHandler.topic[cid] = 1
		else
			npcHandler:say('I\'m sorry but I sell only one copy to each customer. Otherwise they would have been sold out a long time ago.', cid)
		end

	elseif npcHandler.topic[cid] == 1 then
		if msgcontains(msg, 'yes') then
			if not player:removeMoneyNpc(150) then
				npcHandler:say('No gold, no sale, that\'s it.', cid)
				return true
			end

			npcHandler:say('Here you are. Happy cooking!', cid)
			player:setStorageValue(Storage.MaryzaCookbook, 1)
			player:addItem(2347, 1)
		elseif msgcontains(msg, 'no') then
			npcHandler:say('I have but a few copies, anyway.', cid)
		end
	end
	return true
end

npcHandler:setMessage(MESSAGE_GREET, 'Welcome to the Jolly Axeman, |PLAYERNAME|. Have a good time and eat some food!')

npcHandler:setCallback(CALLBACK_MESSAGE_DEFAULT, creatureSayCallback)
local focusModule = FocusModule:new()
focusModule:addGreetMessage('hello maryza')
focusModule:addGreetMessage('hi maryza')
focusModule:addGreetMessage('hello, maryza')
npcHandler:addModule(focusModule)


-- Shop items (from XML parameters)
local shopItems = {
    {id = 2666, buy = 5, sell = 0, subType = 0, name = "meat"},
    {id = 2671, buy = 8, sell = 0, subType = 0, name = "ham"},
    {id = 2687, buy = 2, sell = 0, subType = 0, name = "cookie"},
    {id = 2689, buy = 4, sell = 0, subType = 0, name = "bread"},
    {id = 2690, buy = 2, sell = 0, subType = 0, name = "roll"},
    {id = 2691, buy = 3, sell = 0, subType = 0, name = "brown bread"},
    {id = 2696, buy = 6, sell = 0, subType = 0, name = "cheese"},
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
