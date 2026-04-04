-- Melfar - Converted from XML to Lua NpcType
-- Original XML: data/npc/Melfar.xml
-- Original Script: data/npc/scripts/Melfar.lua

local npcName = "Melfar"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a melfar")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(2000)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 69})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)


local config = {
	{position = Position(32474, 31947, 7), type = 2, description = 'Tree 1'},
	{position = Position(32515, 31927, 7), type = 2, description = 'Tree 2'},
	{position = Position(32458, 31997, 7), type = 2, description = 'Tree 3'}
}

local function creatureSayCallback(cid, type, msg)
	if not npcHandler:isFocused(cid) then
		return false
	end

	local player = Player(cid)
	if(msgcontains(msg, "mission")) then
		if(player:getStorageValue(Storage.TheNewFrontier.Questline) == 4) then
			npcHandler:say({
				"Ha! Men and wood you say? Well, I might be able to relocate some of our miners to the base. Acquiring wood is an entirely different matter though. ... ",
				"I can't spare any men for woodcutting right now but I have an unusual idea that might help. ... ",
				"As you might know, this area is troubled by giant beavers. Once a year, the miners decide to have some fun, so they lure the beavers and jump on them to have some sort of rodeo. ... ",
				"However, I happen to have some beaver bait left from the last year's competition. ... ",
				"If you place it on trees on some strategic locations, we could let the beavers do the work and later on, I'll send men to get the fallen trees. ... ",
				"Does this sound like something you can handle? "
			}, cid)
			npcHandler.topic[cid] = 1
		elseif(player:getStorageValue(Storage.TheNewFrontier.Questline) == 6) then
			npcHandler:say("Yes, I can hear them even from here. It has to be a legion of beavers! I'll send the men to get the wood as soon as their gnawing frenzy has settled! You can report to Ongulf that men and wood will be on their way!", cid)
			player:setStorageValue(Storage.TheNewFrontier.Questline, 7)
			player:setStorageValue(Storage.TheNewFrontier.Mission02, 6) --Questlog, The New Frontier Quest "Mission 02: From Kazordoon With Love"
		end
	elseif(msgcontains(msg, "yes")) then
		if(npcHandler.topic[cid] == 1) then
			npcHandler:say({
				"So take this beaver bait. It will work best on dwarf trees. I'll mark the three trees on your map. Here .. here .. and here! So now mark those trees with the beaver bait. ... ",
				"If you're unlucky enough to meet one of the giant beavers, try to stay calm. Don't do any hectic moves, don't yell, don't draw any weapon, and if you should carry anything wooden on you, throw it away as far as you can. "
			}, cid)
			player:setStorageValue(Storage.TheNewFrontier.Questline, 5)
			player:setStorageValue(Storage.TheNewFrontier.Mission02, 2) --Questlog, The New Frontier Quest "Mission 02: From Kazordoon With Love"
			player:addItem(11100, 1)
			for i = 1, #config do
				player:addMapMark(config[i].position, config[i].type, config[i].description)
			end
			npcHandler.topic[cid] = 0
		elseif npcHandler.topic[cid] == 2 then
			if player:removeMoneyNpc(100) then
				player:addItem(11100, 1)
				npcHandler:say("Here you go.", cid)
				npcHandler.topic[cid] = 0
			else
				npcHandler:say("You dont have enough of gold coins.", cid)
				npcHandler.topic[cid] = 0
			end
		end
	elseif msgcontains(msg, "buy flask") or msgcontains(msg, "flask") then
		if player:getStorageValue(Storage.TheNewFrontier.Questline) == 5 then
			npcHandler:say("You want to buy a Flask with Beaver Bait for 100 gold coins?", cid)
			npcHandler.topic[cid] = 2
		else
			npcHandler:say("Im out of stock.", cid)
		end
	elseif msgcontains(msg, "trade") then
		return false -- Let keyword handler manage this
	end
	return true
end

-- Shop items
local shopItems = {
	{id = 12407, buy = 0, sell = 30, name = "broken crossbow"},
	{id = 12428, buy = 0, sell = 75, name = "minotaur horn"},
	{id = 12439, buy = 0, sell = 20, name = "piece of archer armor"},
	{id = 12438, buy = 0, sell = 50, name = "piece of warrior armor"},
	{id = 12429, buy = 0, sell = 110, name = "purple robe"},
	{id = 11100, buy = 100, sell = 0, name = "flask with beaver bait"} -- Always in list, but controlled in onBuyItem
}

local shopItemsById = {}

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
for _, item in ipairs(shopItems) do
	shopItemsById[item.id] = item
end

local function openTradeWindow(cid, message, keywords, parameters, node)
	if not npcHandler:isFocused(cid) then return false end
	local player = Player(cid)
	if not player then return false end
	local npc = Npc(getNpcCid())
	
	local shopList = {}
	for _, item in ipairs(shopItems) do
		-- Only show flask if player is on the mission
		if item.id == 11100 then
			if player:getStorageValue(Storage.TheNewFrontier.Questline) == 5 then
				table.insert(shopList, {id = item.id, buy = item.buy, sell = item.sell, subType = 0, name = item.name})
			end
		else
			table.insert(shopList, {id = item.id, buy = item.buy, sell = item.sell, subType = 0, name = item.name})
		end
	end
	npc:openShopWindow(player, shopList, function() return true end, function() return true end)
	npcHandler:say('I buy broken equipment and sell beaver bait during missions.', cid)
	return true
end

keywordHandler:addKeyword({'trade'}, openTradeWindow, {npcHandler = npcHandler})

npcHandler:setCallback(CALLBACK_MESSAGE_DEFAULT, creatureSayCallback)
npcHandler:setMessage(MESSAGE_GREET, "Greetings, |PLAYERNAME|. Welcome to the mines.")
npcHandler:setMessage(MESSAGE_FAREWELL, "Good bye and be careful in the tunnels.")
npcHandler:setMessage(MESSAGE_SENDTRADE, "Of course, just browse through my wares.")


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
	
	-- Check if trying to buy flask without being on mission
	if itemId == 11100 and player:getStorageValue(Storage.TheNewFrontier.Questline) ~= 5 then
		player:sendCancelMessage("I'm out of stock on that item.")
		return false
	end
	
	local totalCost = amount * shopItem.buy
	if player:getTotalMoney() < totalCost then
		player:sendCancelMessage("You don't have enough money.")
		return false
	end
	local bought = doNpcSellItem(player:getId(), itemId, amount, shopItem.subType or 1, ignoreCap, inBackpacks, ITEM_BACKPACK)
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
	
	local itemSubType = shopItem.subType or 0
	if not ItemType(itemId):isFluidContainer() then
		itemSubType = -1
	end

	if player:removeItem(itemId, amount, itemSubType, ignoreEquipped) then
		player:addMoney(amount * shopItem.sell)
		player:sendTextMessage(MESSAGE_INFO_DESCR, "Sold " .. amount .. "x " .. shopItem.name .. " for " .. (amount * shopItem.sell) .. " gold.")
		return true
	end
	player:sendCancelMessage("You do not have this object.")
	return false
end)

npcHandler:addModule(FocusModule:new())
npcType:register()
