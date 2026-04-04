-- Olrik - Converted from XML to Lua NpcType
-- Original XML: data/npc/Olrik.xml
-- Original Script: data/npc/scripts/Olrik.lua

local npcName = "Olrik"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a olrik")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(2000)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 128, lookHead = 115, lookBody = 79, lookLegs = 117, lookFeet = 76})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)


local function creatureSayCallback(cid, type, msg)
	if not npcHandler:isFocused(cid) then
		return false
	end
	local player = Player(cid)
	if msgcontains(msg, "measurements") then
		if player:getStorageValue(Storage.postman.Mission07) >= 1 and	player:getStorageValue(Storage.postman.MeasurementsOlrik) ~= 1 then
			npcHandler:say("My measurements? Listen, lets make that a bit more exciting ... No, no, not what you think! I mean let's gamble. I will roll a dice. If I roll a 6 you win and I'll tell you what you need to know, else I win and get 5 gold. Deal? ", cid)
			npcHandler.topic[cid] = 1
	else
			npcHandler:say("...", cid)
			npcHandler.topic[cid] = 0
		end
	elseif msgcontains(msg, "yes") then
		if player:getMoney() + player:getBankBalance() >= 5 then
			player:removeMoneyNpc(5)
			local number = math.random(6)
			if number ~= 6 then
				npcHandler:say("Ok, here we go ... " .. number .. "! You lose! Try again.", cid)
			else
				npcHandler:say("Ok, here we go ... " .. number .. "! You have won! How lucky you are! So listen ...<tells you what you need to know> ", cid)
				player:setStorageValue(Storage.postman.Mission07, player:getStorageValue(Storage.postman.Mission07) + 1)
				player:setStorageValue(Storage.postman.MeasurementsOlrik, 1)
				npcHandler.topic[cid] = 0
			end
		end
	end
	return true
end

npcHandler:setCallback(CALLBACK_MESSAGE_DEFAULT, creatureSayCallback)
npcHandler:setMessage(MESSAGE_GREET, "Hello. How may I help you |PLAYERNAME|? Ask me for a {trade} if you want to buy something. I can also explain the {mail} system.")
npcHandler:setMessage(MESSAGE_FAREWELL, "It was a pleasure to help you, |PLAYERNAME|.")
npcHandler:setMessage(MESSAGE_SENDTRADE, "Here. Don't forget that you need to buy a label too if you want to send a parcel. Always write the name of the {receiver} in the first line.")


-- Shop items (from XML parameters)
local shopItems = {
    {id = 2595, buy = 15, sell = 0, subType = 0, name = "parcel"},
    {id = 2597, buy = 8, sell = 0, subType = 0, name = "letter"},
    {id = 2599, buy = 1, sell = 0, subType = 0, name = "label"},
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
