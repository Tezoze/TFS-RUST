-- Captain Max - Converted from XML to Lua NpcType
-- Original XML: data/npc/Captain Max.xml
-- Original Script: data/npc/scripts/Captain Max.lua

local npcName = "Captain Max"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a captain max")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(2000)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 134, lookHead = 95, lookBody = 10, lookLegs = 56, lookFeet = 77})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)


local voices = { {text = 'Whoah. That was a large shadow passing by.'} }
npcHandler:addModule(VoiceModule:new(voices))



local function addTravelKeyword(keyword, text, cost, destination, condition)
	-- Create a single keyword with condition checking inside the callback
	local travelNode = keywordHandler:addKeyword({keyword}, function(cid, message, keywords, parameters, node)
		local player = Player(cid)
		if not player then return false end
		
		-- Check condition - if condition returns true, reject travel
		if condition and condition(player) then
			npcHandler:say('I\'m sorry but I won\'t take anyone there without the permission of the explorer society. You should talk to Wyrdin in Edron.', cid)
			-- Reset to root so yes/no children don't trigger
			keywordHandler:reset(cid)
			return true
		end
		
		-- Condition passed - offer travel
		npcHandler:say(text, cid)
		return true
	end, {npcHandler = npcHandler})
	
	-- Add yes/no responses for travel confirmation
	travelNode:addChildKeyword({'yes'}, StdModule.travel, {npcHandler = npcHandler, premium = false, text = 'Hold on!', cost = cost, discount = 'postman', destination = destination})
	travelNode:addChildKeyword({'no'}, StdModule.say, {npcHandler = npcHandler, text = 'You shouldn\'t miss the experience.', reset = true})
end

addTravelKeyword('calassa', 'Should I bring you to Calassa for 200 gold?', 200, Position(31911, 32710, 6), function(player) return player:getStorageValue(Storage.ExplorerSociety.CalassaQuest) < 1 end)
addTravelKeyword('yalahar', 'That is quite a long unprofitable travel. I\'ll bring you to Yalahar for 400 gold though. Is that ok with you?', 400, Position(32816, 31272, 6), function(player) return player:getStorageValue(Storage.TheWayToYalahar.QuestLine) < 1 end)

keywordHandler:addKeyword({'sail'}, StdModule.say, {npcHandler = npcHandler, text = 'Welcome on board, noble |PLAYERNAME|. I can bring you to {Calassa} or {Yalahar}, but only if you have the according mission from {Berenice} or {Wyrdin}.'})
keywordHandler:addKeyword({'passage'}, StdModule.say, {npcHandler = npcHandler, text = 'Welcome on board, noble |PLAYERNAME|. I can bring you to {Calassa} or {Yalahar}, but only if you have the according mission from {Berenice} or {Wyrdin}.'})
keywordHandler:addKeyword({'job'}, StdModule.say, {npcHandler = npcHandler, text = 'I am the captain of this ship.'})
keywordHandler:addKeyword({'captain'}, StdModule.say, {npcHandler = npcHandler, text = 'I am the captain of this ship.'})

npcHandler:setMessage(MESSAGE_GREET, "Ahoy, |PLAYERNAME|. On a mission for the explorer society, eh?")
npcHandler:setMessage(MESSAGE_FAREWELL, "Good bye.")
npcHandler:setMessage(MESSAGE_WALKAWAY, "Good bye.")


-- Shop items (from XML parameters)
local shopItems = {
    {id = 5461, buy = 5000, sell = 5000, subType = 0, name = "helmet of the deep"},
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
npcType:onSellItem(function(npc, player, itemId, subType, amount, ignoreCap, name, totalCost)
    local shopItem = getShopItem(itemId, subType, false)
    if not shopItem or shopItem.sell <= 0 then return false end
    local sold = player:removeItem(itemId, amount)
    if not sold then
        player:sendCancelMessage("You don't have that item.")
        return false
    end
    local totalPrice = amount * shopItem.sell
    player:addMoney(totalPrice)
    player:sendTextMessage(MESSAGE_INFO_DESCR, "Sold " .. amount .. "x " .. shopItem.name .. " for " .. totalPrice .. " gold.")
    return true
end)

npcHandler:addModule(FocusModule:new())
npcType:register()
