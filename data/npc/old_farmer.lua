-- Old Farmer - Converted from XML to Lua NpcType
-- Original XML: data/npc/Old Farmer.xml
-- Original Script: data/npc/scripts/Old Farmer.lua

local npcName = "Old Farmer"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a old farmer")
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

npcHandler:setMessage(MESSAGE_GREET, 'Welcome to the Creature Products Supermarket, |PLAYERNAME|. Have a good time!')

npcHandler:setCallback(CALLBACK_MESSAGE_DEFAULT, creatureSayCallback)
local focusModule = FocusModule:new()
focusModule:addGreetMessage('hi')
npcHandler:addModule(focusModule)


-- Shop items (from XML parameters)
local shopItems = {
    {id = 12403, buy = 0, sell = 290, subType = 0, name = "battle stone"},
    {id = 10550, buy = 0, sell = 100, subType = 0, name = "bloody pincers"},
    {id = 12658, buy = 0, sell = 380, subType = 0, name = "brimstone fangs"},
    {id = 12659, buy = 0, sell = 210, subType = 0, name = "brimstone shell"},
    {id = 12408, buy = 0, sell = 35, subType = 0, name = "broken shamanic staff"},
    {id = 11219, buy = 0, sell = 45, subType = 0, name = "compass"},
    {id = 10555, buy = 0, sell = 280, subType = 0, name = "cultish mask"},
    {id = 10556, buy = 0, sell = 150, subType = 0, name = "cultish robe"},
    {id = 10574, buy = 0, sell = 55, subType = 0, name = "cyclops toe"},
    {id = 5954, buy = 0, sell = 1000, subType = 0, name = "demon horn"},
    {id = 10564, buy = 0, sell = 80, subType = 0, name = "demonic skeletal hand"},
    {id = 12614, buy = 0, sell = 550, subType = 0, name = "draken sulphur"},
    {id = 12420, buy = 0, sell = 50, subType = 0, name = "elven scouting glass"},
    {id = 10552, buy = 0, sell = 45, subType = 0, name = "elvish talisman"},
    {id = 10553, buy = 0, sell = 375, subType = 0, name = "fiery heart"},
    {id = 12422, buy = 0, sell = 30, subType = 0, name = "flask of embalming fluid"},
    {id = 10578, buy = 0, sell = 280, subType = 0, name = "frosty heart"},
    {id = 5877, buy = 0, sell = 100, subType = 0, name = "green dragon leather"},
    {id = 5920, buy = 0, sell = 100, subType = 0, name = "green dragon scale"},
    {id = 11221, buy = 0, sell = 475, subType = 0, name = "hellspawn tail"},
    {id = 10608, buy = 0, sell = 60, subType = 0, name = "lion's mane"},
    {id = 11215, buy = 0, sell = 320, subType = 0, name = "metal spike"},
    {id = 10577, buy = 0, sell = 700, subType = 0, name = "mystical hourglass"},
    {id = 11113, buy = 0, sell = 150, subType = 0, name = "orc tooth"},
    {id = 11337, buy = 0, sell = 250, subType = 0, name = "petrified scream"},
    {id = 10580, buy = 0, sell = 420, subType = 0, name = "piece of dead brain"},
    {id = 10558, buy = 0, sell = 45, subType = 0, name = "piece of scarab shell"},
    {id = 12440, buy = 0, sell = 25, subType = 0, name = "pile of grave earth"},
    {id = 10557, buy = 0, sell = 50, subType = 0, name = "poisonous slime"},
    {id = 10567, buy = 0, sell = 30, subType = 0, name = "polar bear paw"},
    {id = 12400, buy = 0, sell = 60, subType = 0, name = "protective charm"},
    {id = 12448, buy = 0, sell = 66, subType = 0, name = "rope belt"},
    {id = 11228, buy = 0, sell = 400, subType = 0, name = "sabretooth"},
    {id = 10611, buy = 0, sell = 400, subType = 0, name = "snake skin"},
    {id = 11226, buy = 0, sell = 600, subType = 0, name = "strand of medusa hair"},
    {id = 10603, buy = 0, sell = 20, subType = 0, name = "swamp grass"},
    {id = 11224, buy = 0, sell = 150, subType = 0, name = "thick fur"},
    {id = 10602, buy = 0, sell = 275, subType = 0, name = "vampire teeth"},
    {id = 10571, buy = 0, sell = 460, subType = 0, name = "war crystal"},
    {id = 11322, buy = 0, sell = 200, subType = 0, name = "warmaster's wristguards"},
    {id = 11212, buy = 0, sell = 20, subType = 0, name = "winter wolf fur"},
    {id = 10582, buy = 0, sell = 400, subType = 0, name = "wyrm scale"},
    {id = 10561, buy = 0, sell = 265, subType = 0, name = "wyvern talisman"},
    {id = 12403, buy = 580, sell = 0, subType = 0, name = "battle stone"},
    {id = 10550, buy = 200, sell = 0, subType = 0, name = "bloody pincers"},
    {id = 12658, buy = 760, sell = 0, subType = 0, name = "brimstone fangs"},
    {id = 12659, buy = 420, sell = 0, subType = 0, name = "brimstone shell"},
    {id = 12408, buy = 70, sell = 0, subType = 0, name = "broken shamanic staff"},
    {id = 11219, buy = 90, sell = 0, subType = 0, name = "compass"},
    {id = 10555, buy = 560, sell = 0, subType = 0, name = "cultish mask"},
    {id = 10556, buy = 300, sell = 0, subType = 0, name = "cultish robe"},
    {id = 10574, buy = 110, sell = 0, subType = 0, name = "cyclops toe"},
    {id = 5954, buy = 2000, sell = 0, subType = 0, name = "demon horn"},
    {id = 10564, buy = 160, sell = 0, subType = 0, name = "demonic skeletal hand"},
    {id = 12614, buy = 1100, sell = 0, subType = 0, name = "draken sulphur"},
    {id = 12420, buy = 100, sell = 0, subType = 0, name = "elven scouting glass"},
    {id = 10552, buy = 90, sell = 0, subType = 0, name = "elvish talisman"},
    {id = 10553, buy = 750, sell = 0, subType = 0, name = "fiery heart"},
    {id = 12422, buy = 60, sell = 0, subType = 0, name = "flask of embalming fluid"},
    {id = 10578, buy = 560, sell = 0, subType = 0, name = "frosty heart"},
    {id = 5877, buy = 200, sell = 0, subType = 0, name = "green dragon leather"},
    {id = 5920, buy = 200, sell = 0, subType = 0, name = "green dragon scale"},
    {id = 11221, buy = 950, sell = 0, subType = 0, name = "hellspawn tail"},
    {id = 10608, buy = 120, sell = 0, subType = 0, name = "lion's mane"},
    {id = 11215, buy = 640, sell = 0, subType = 0, name = "metal spike"},
    {id = 10577, buy = 1400, sell = 0, subType = 0, name = "mystical hourglass"},
    {id = 11113, buy = 300, sell = 0, subType = 0, name = "orc tooth"},
    {id = 11337, buy = 500, sell = 0, subType = 0, name = "petrified scream"},
    {id = 10580, buy = 840, sell = 0, subType = 0, name = "piece of dead brain"},
    {id = 10558, buy = 90, sell = 0, subType = 0, name = "piece of scarab shell"},
    {id = 12440, buy = 50, sell = 0, subType = 0, name = "pile of grave earth"},
    {id = 10557, buy = 100, sell = 0, subType = 0, name = "poisonous slime"},
    {id = 10567, buy = 60, sell = 0, subType = 0, name = "polar bear paw"},
    {id = 12400, buy = 120, sell = 0, subType = 0, name = "protective charm"},
    {id = 12448, buy = 132, sell = 0, subType = 0, name = "rope belt"},
    {id = 11228, buy = 800, sell = 0, subType = 0, name = "sabretooth"},
    {id = 10611, buy = 800, sell = 0, subType = 0, name = "snake skin"},
    {id = 11226, buy = 1200, sell = 0, subType = 0, name = "strand of medusa hair"},
    {id = 10603, buy = 40, sell = 0, subType = 0, name = "swamp grass"},
    {id = 11224, buy = 300, sell = 0, subType = 0, name = "thick fur"},
    {id = 10602, buy = 550, sell = 0, subType = 0, name = "vampire teeth"},
    {id = 10571, buy = 920, sell = 0, subType = 0, name = "war crystal"},
    {id = 11322, buy = 400, sell = 0, subType = 0, name = "warmaster's wristguards"},
    {id = 11212, buy = 40, sell = 0, subType = 0, name = "winter wolf fur"},
    {id = 10582, buy = 800, sell = 0, subType = 0, name = "wyrm scale"},
    {id = 10561, buy = 530, sell = 0, subType = 0, name = "wyvern talisman"},
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
