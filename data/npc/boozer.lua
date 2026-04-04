-- Boozer - Converted from XML to Lua NpcType
-- Original XML: data/npc/Boozer.xml
-- Original Script: data/npc/scripts/Boozer.lua

local npcName = "Boozer"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a boozer")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(2000)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 128, lookHead = 76, lookBody = 20, lookLegs = 116, lookFeet = 76})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)


local function creatureSayCallback(cid, type, msg)
	if not npcHandler:isFocused(cid) then
		return false
	end

	local player = Player(cid)
	if msgcontains(msg, "mission") then
		if player:getStorageValue(Storage.TibiaTales.ultimateBoozeQuest) == 2 and player:removeItem(7495, 1) then
			player:setStorageValue(Storage.TibiaTales.ultimateBoozeQuest, 3)
			npcHandler.topic[cid] = 0
			player:addItem(5710, 1)
			player:addItem(2152, 10)
			player:addExperience(100, true)
			npcHandler:say("Yessss! Now I only need to build my own small brewery, figure out the secret recipe, duplicate the dwarvish brew and BANG I'll be back in business! Here take this as a reward.", cid)
		elseif player:getStorageValue(Storage.TibiaTales.ultimateBoozeQuest) < 1 then
			npcHandler.topic[cid] = 1
			npcHandler:say("Shush!! I don't want everybody to know what I am up to. Listen, things are not going too well, I need a new attraction. Do you want to help me?", cid)
		end
	elseif msgcontains(msg, "dwarven brown ale") or msgcontains(msg, "cask of brown ale") then
		-- Beregar quest: dwarven brown ale sale
		if player:getStorageValue(Storage.hiddenCityOfBeregar.TheGoodGuard) == 1 and player:getStorageValue(Storage.TibiaTales.ultimateBoozeQuest) == 3 then
			npcHandler:say("You are soooo lucky. Only recently I finished my first cask. As this would never have been possible without you, I make you a special offer. 3000 Gold! Alright?", cid)
			npcHandler.topic[cid] = 2
		elseif player:getStorageValue(Storage.TibiaTales.ultimateBoozeQuest) ~= 3 then
			npcHandler:say("I haven't finished brewing any dwarven brown ale yet. Come back later!", cid)
		else
			npcHandler:say("I don't have any more dwarven brown ale available right now.", cid)
		end
	elseif msgcontains(msg, "yes") then
		if npcHandler.topic[cid] == 1 then
			player:setStorageValue(Storage.TibiaTales.DefaultStart, 1)
			player:setStorageValue(Storage.TibiaTales.ultimateBoozeQuest, 1)
			player:addItem(7496, 1)
			npcHandler:say("Good! Listen closely. Take this bottle and go to Kazordoon. I need a sample of their very special brown ale. You may find a cask in their brewery. Come back as soon as you got it.", cid)
		elseif npcHandler.topic[cid] == 2 then
			-- Sell dwarven brown ale for 3000 gp
			if player:getMoney() >= 3000 then
				player:removeMoney(3000)
				player:addItem(9689, 1) -- cask of brown ale
				npcHandler:say("Here it is. Have fun with this delicious brew.", cid)
				npcHandler.topic[cid] = 0
			else
				npcHandler:say("You don't have enough money. I need 3000 gold coins.", cid)
			end
		end
	elseif msgcontains(msg, "no") then
		if npcHandler.topic[cid] == 2 then
			npcHandler:say("Maybe another time then.", cid)
			npcHandler.topic[cid] = 0
		end
	end
	return true
end

npcHandler:setCallback(CALLBACK_MESSAGE_DEFAULT, creatureSayCallback)


-- Shop items (from XML parameters)
local shopItems = {
    {id = 2006, buy = 10, sell = 0, subType = 15, name = "wine"},
    {id = 2689, buy = 4, sell = 0, subType = 0, name = "bread"},
    {id = 9689, buy = 3000, sell = 0, subType = 0, name = "cask of brown ale"},
    {id = 2696, buy = 6, sell = 0, subType = 0, name = "cheese"},
    {id = 2687, buy = 5, sell = 0, subType = 0, name = "cookie"},
    {id = 2671, buy = 8, sell = 0, subType = 0, name = "ham"},
    {id = 2666, buy = 5, sell = 0, subType = 0, name = "meat"},
    {id = 2012, buy = 3, sell = 0, subType = 3, name = "mug of beer"},
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
