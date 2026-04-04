-- Telas - Converted from XML to Lua NpcType
-- Original XML: data/npc/Telas.xml
-- Original Script: data/npc/scripts/Telas.lua

local npcName = "Telas"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a telas")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(2000)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 133, lookHead = 39, lookFeet = 76, lookAddons = 1})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)


local function creatureSayCallback(cid, type, msg)
	if not npcHandler:isFocused(cid) then
		return false
	end

	local player = Player(cid)
	if(msgcontains(msg, "farmine")) then
		if(player:getStorageValue(Storage.TheNewFrontier.Questline) == 15) then
			npcHandler:say("I have heard only little about this mine. I am a bit absorbed in my studies. But what does this mine have to do with me?", cid)
			npcHandler.topic[cid] = 1
		end
	elseif(msgcontains(msg, "reason")) then
		if(npcHandler.topic[cid] == 1) then
			if(player:getStorageValue(Storage.TheNewFrontier.BribeTelas) < 1) then
				npcHandler:say("Well it sounds like a good idea to test my golems in some real environment. I think it is acceptable to send some of them to Farmine.", cid)
				player:setStorageValue(Storage.TheNewFrontier.BribeTelas, 1)
				player:setStorageValue(Storage.TheNewFrontier.Mission05, player:getStorageValue(Storage.TheNewFrontier.Mission05) + 1) --Questlog, The New Frontier Quest "Mission 05: Getting Things Busy"
			end
		end
	end
	return true
end

npcHandler:setCallback(CALLBACK_MESSAGE_DEFAULT, creatureSayCallback)


-- Shop items (from XML parameters)
local shopItems = {
    {id = 10549, buy = 0, sell = 200, subType = 0, name = "ancient stone"},
    {id = 12403, buy = 0, sell = 290, subType = 0, name = "battle stone"},
    {id = 10573, buy = 0, sell = 190, subType = 0, name = "broken gladiator shield"},
    {id = 9942, buy = 0, sell = 1000, subType = 0, name = "crystal of balance"},
    {id = 9941, buy = 0, sell = 2000, subType = 0, name = "crystal of focus"},
    {id = 9980, buy = 0, sell = 3000, subType = 0, name = "crystal of power"},
    {id = 10572, buy = 0, sell = 200, subType = 0, name = "gear crystal"},
    {id = 9690, buy = 0, sell = 500, subType = 0, name = "gear wheel"},
    {id = 5892, buy = 0, sell = 15000, subType = 0, name = "huge chunk of crude iron"},
    {id = 11215, buy = 0, sell = 320, subType = 0, name = "metal spike"},
    {id = 5889, buy = 0, sell = 3000, subType = 0, name = "piece of draconian steel"},
    {id = 5888, buy = 0, sell = 500, subType = 0, name = "piece of hell steel"},
    {id = 10581, buy = 0, sell = 550, subType = 0, name = "piece of hellfire armor"},
    {id = 5887, buy = 0, sell = 10000, subType = 0, name = "piece of royal steel"},
    {id = 11227, buy = 0, sell = 500, subType = 0, name = "shiny stone"},
    {id = 11232, buy = 0, sell = 100, subType = 0, name = "sulphurous stone"},
    {id = 10571, buy = 0, sell = 460, subType = 0, name = "war crystal"},
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
