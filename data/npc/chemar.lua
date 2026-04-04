-- Chemar - Converted from XML to Lua NpcType
-- Original XML: data/npc/Chemar.xml
-- Original Script: data/npc/scripts/Chemar.lua

local npcName = "Chemar"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a chemar")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(2000)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 130, lookHead = 95, lookBody = 3, lookLegs = 14, lookFeet = 76})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)


local voices = {
	{ text = 'Ask me if you need letters or parcels. I\'ll deliver them via airmail, of course!' },
	{ text = 'Feel the wind in your hair during one of my carpet rides!' }
}

npcHandler:addModule(VoiceModule:new(voices))


local function creatureSayCallback(cid, type, msg)
	if not npcHandler:isFocused(cid) then
		return false
	end

	local player = Player(cid)
	if msgcontains(msg, 'fly') then
			npcHandler:say('The different places we travel to are: {darashia}, {svargrond}, {femor hills}, {edron}, {Kazordoon}', cid)
			return true
	end
	return true
end


-- Travel
local function addTravelKeyword(keyword, cost, destination)
	local travelKeyword = keywordHandler:addKeyword({keyword}, StdModule.say, {npcHandler = npcHandler, text = 'Do you seek a ride to ' .. keyword:titleCase() .. ' for |TRAVELCOST|?', cost = cost, discount = 'postman'})
		travelKeyword:addChildKeyword({'yes'}, StdModule.travel, {npcHandler = npcHandler, premium = false, text = 'Hold on!', cost = cost, discount = 'postman', destination = destination})
		travelKeyword:addChildKeyword({'no'}, StdModule.say, {npcHandler = npcHandler, text = 'You shouldn\'t miss the experience.', reset = true})
end

-- Special handling for Farmine
local farmineKeyword = keywordHandler:addKeyword({'farmine'}, function(cid, type, msg, matches, node)
    local player = Player(cid)
    if not player then
        npcHandler:say("Something went wrong. Please try again.", cid)
        return true
    end
    local accessValue = player:getStorageValue(Storage.TheNewFrontier.Mission10) or -1
    if accessValue < 1 then
        npcHandler:say('Never heard about a place like this.', cid)
        return true
    else
        npcHandler:say('Do you seek a ride to Farmine for 60 gold pieces?', cid)
        npcHandler.topic[cid] = 1 -- Set topic to allow child keywords
        return true
    end
end, {npcHandler = npcHandler})

-- Child keywords for Farmine travel
farmineKeyword:addChildKeyword({'yes'}, StdModule.travel, {npcHandler = npcHandler, premium = false, text = 'Hold on!', cost = 60, discount = 'postman', destination = Position(32983, 31539, 1)})
farmineKeyword:addChildKeyword({'no'}, StdModule.say, {npcHandler = npcHandler, text = 'You shouldn\'t miss the experience.', reset = true})

addTravelKeyword('edron', 40, Position(33193, 31784, 3))
addTravelKeyword('svargrond', 60, Position(32253, 31097, 4))
addTravelKeyword('femor hills', 60, Position(32536, 31837, 4))
addTravelKeyword('kazordoon', 80, Position(32588, 31941, 0))
addTravelKeyword('hills', 60, Position(32536, 31837, 4))

npcHandler:setMessage(MESSAGE_GREET, 'Daraman\'s blessings, traveller |PLAYERNAME|.')
npcHandler:setMessage(MESSAGE_FAREWELL, 'It was a pleasure to help you, |PLAYERNAME|.')
npcHandler:setMessage(MESSAGE_WALKAWAY, 'It was a pleasure to help you, |PLAYERNAME|.')

npcHandler:setCallback(CALLBACK_MESSAGE_DEFAULT, creatureSayCallback)


-- Shop items (from XML parameters)
local shopItems = {
    {id = 2599, buy = 1, sell = 0, subType = 0, name = "label"},
    {id = 2595, buy = 15, sell = 0, subType = 0, name = "parcel"},
    {id = 2597, buy = 8, sell = 0, subType = 0, name = "letter"},
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
