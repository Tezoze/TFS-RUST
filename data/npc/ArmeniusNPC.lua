-- ArmeniusNPC - Converted from XML to Lua NpcType
-- Original XML: data/npc/ArmeniusNPC.xml
-- Original Script: data/npc/scripts/ArmeniusNPC.lua

local npcId = "ArmeniusNPC"  -- Unique ID for spawn system
local npcDisplayName = "Armenius"  -- Name shown in-game
local npcType = Game.createNpcType(npcId)

-- NPC Properties (from XML)
npcType:name(npcDisplayName)
npcType:nameDescription("a armenius")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(2000)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 132, lookHead = 114, lookBody = 78, lookLegs = 113})
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
				player:setStorageValue(Storage.BloodBrothers.ArmeniusSuspect, 1)
				npcHandler:say("A cookie? *looks suspicious* No thank you, I... I don't eat such things. Keep it away from me!", cid)
			else
				npcHandler:say("I don't see any cookie, and I wouldn't want one anyway.", cid)
			end
		else
			npcHandler:say("I have no interest in your cookies.", cid)
		end
	
	-- Handle "alori mort" spell
	elseif msgcontains(msg, "alori mort") then
		if player:getStorageValue(Storage.BloodBrothers.Mission03) == 1 then
			player:setStorageValue(Storage.BloodBrothers.Mission03, 2)
			
			-- Small chance for "His True Face" achievement
			if math.random(1, 100) <= 10 then -- 10% chance
				player:setStorageValue(Storage.BloodBrothers.HisTrueFace, 1)
				player:addAchievement("His True Face")
				npcHandler:say("NOOOO! How did you... *transforms into hideous vampire form* You will pay for this!", cid)
				-- Teleport player to Armenius's basement
				player:teleportTo(Position(32856, 31323, 8))
				player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "Armenius has shown his true vampire form!")
			else
				npcHandler:say("What?! What are you doing?! *looks around nervously* I... I don't know what you're talking about!", cid)
			end
		else
			npcHandler:say("What strange words are you speaking?", cid)
		end
	elseif msgcontains(msg, "blood crystal") then
		npcHandler:say("If you want blood, go kill a pig.", cid)
		return true
	end

	return true
end

npcHandler:setMessage(MESSAGE_GREET, "What do you want from me, |PLAYERNAME|?")
npcHandler:setMessage(MESSAGE_FAREWELL, "Finally... leave me be.")
npcHandler:setMessage(MESSAGE_WALKAWAY, "Good riddance.")

npcHandler:setCallback(CALLBACK_MESSAGE_DEFAULT, creatureSayCallback)


-- Shop items (from XML parameters)
local shopItems = {
    {id = 2666, buy = 5, sell = 0, subType = 0, name = "meat"},
    {id = 2696, buy = 5, sell = 0, subType = 0, name = "cheese"},
    {id = 2671, buy = 8, sell = 0, subType = 0, name = "ham"},
    {id = 2689, buy = 3, sell = 0, subType = 0, name = "bread"},
    {id = 2695, buy = 2, sell = 0, subType = 0, name = "egg"},
    {id = 2685, buy = 3, sell = 0, subType = 0, name = "tomato"},
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
