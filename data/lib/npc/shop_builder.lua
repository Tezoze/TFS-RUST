-- ShopBuilder: Specialized NPC builder for buy/sell trade NPCs
-- Extends NpcBuilder with shop item management, trade window, and buy/sell callbacks.
-- Automatically sets SPEECHBUBBLE_TRADE and wires onBuyItem/onSellItem on the NpcType.

ShopBuilder = setmetatable({}, { __index = NpcBuilder })
ShopBuilder.__index = ShopBuilder

function ShopBuilder:new(name, outfit)
    local obj = NpcBuilder.new(self, name, outfit)
    obj._shopItems = {}  -- { {id, buy, sell, subType, name}, ... }
    obj._tradeMsg = "Of course, just browse through my wares."
    obj._onEndTradeCallback = nil
    setmetatable(obj, self)
    return obj
end

function ShopBuilder:tradeMessage(msg) self._tradeMsg = msg; return self end

function ShopBuilder:addBuyable(name, itemId, buyPrice, subType)
    self._shopItems[#self._shopItems + 1] = {
        id = itemId, buy = buyPrice, sell = 0,
        subType = subType or 1, name = name
    }
    return self
end

function ShopBuilder:addSellable(name, itemId, sellPrice, subType)
    -- Check if item already exists, merge sell price
    for _, item in ipairs(self._shopItems) do
        if item.id == itemId and item.subType == (subType or 0) then
            item.sell = sellPrice
            return self
        end
    end
    self._shopItems[#self._shopItems + 1] = {
        id = itemId, buy = 0, sell = sellPrice,
        subType = subType or 0, name = name
    }
    return self
end

function ShopBuilder:addBuyableAndSellable(name, itemId, buyPrice, sellPrice, subType)
    self._shopItems[#self._shopItems + 1] = {
        id = itemId, buy = buyPrice, sell = sellPrice,
        subType = subType or 1, name = name
    }
    return self
end

function ShopBuilder:onNpcAppear(npc)
    npc:setSpeechBubble(SPEECHBUBBLE_TRADE)
end

function ShopBuilder:findShopItem(itemId, subType, isBuying)
    local itemType = ItemType(itemId)
    if itemType:isFluidContainer() then
        for _, item in ipairs(self._shopItems) do
            if item.id == itemId and item.subType == subType then
                return item
            end
        end
    end
    for _, item in ipairs(self._shopItems) do
        if item.id == itemId then
            if isBuying and item.buy > 0 then return item end
            if not isBuying and item.sell > 0 then return item end
        end
    end
    return nil
end

function ShopBuilder:handleBuyItem(npc, player, itemId, subType, amount, ignoreCap, inBackpacks)
    local shopItem = self:findShopItem(itemId, subType, true)
    if not shopItem or shopItem.buy <= 0 then return false end
    local totalCost = amount * shopItem.buy
    if player:getTotalMoney() < totalCost then
        player:sendCancelMessage("You don't have enough money.")
        return false
    end
    local bought = doNpcSellItem(player:getId(), itemId, amount,
        shopItem.subType, ignoreCap, inBackpacks, ITEM_BACKPACK)
    if bought == 0 then
        player:sendCancelMessage("You do not have enough capacity.")
        return false
    end
    player:removeTotalMoney(bought * shopItem.buy)
    player:sendTextMessage(MESSAGE_INFO_DESCR,
        "Bought " .. bought .. "x " .. shopItem.name ..
        " for " .. (bought * shopItem.buy) .. " gold.")
    return true
end

function ShopBuilder:handleSellItem(npc, player, itemId, subType, amount, ignoreEquipped)
    local shopItem = self:findShopItem(itemId, subType, false)
    if not shopItem or shopItem.sell <= 0 then return false end
    local itemSubType = shopItem.subType or 0
    if not ItemType(itemId):isFluidContainer() then
        itemSubType = -1
    end
    if player:removeItem(itemId, amount, itemSubType, ignoreEquipped) then
        player:addMoney(amount * shopItem.sell)
        player:sendTextMessage(MESSAGE_INFO_DESCR,
            "Sold " .. amount .. "x " .. shopItem.name ..
            " for " .. (amount * shopItem.sell) .. " gold.")
        return true
    end
    player:sendCancelMessage("You do not have this object.")
    return false
end


function ShopBuilder:addBuyableItemContainer(name, containerId, itemId, cost, subType)
    local itemName = name
    local containerItemId = containerId
    local contentItemId = itemId
    local unitCost = cost
    local contentSubType = subType or 1

    -- Register "buy <name>" keyword
    self._keywords:addKeyword({"buy", itemName:lower()}, function(npc, player, message, builder)
        if not builder:isFocused(npc:getId(), player:getId()) then return false end
        local s = InstanceState.get(npc:getId(), player:getId())
        if not s then return false end

        -- Parse quantity from message (e.g., "buy 5 backpack of runes")
        local amount = 1
        local countMatch = message:match("%d+")
        if countMatch then
            amount = math.max(1, math.min(100, tonumber(countMatch)))
        end

        s.topic = "container_buy_confirm"
        s.shopSelection = {
            name = itemName,
            containerId = containerItemId,
            itemId = contentItemId,
            cost = unitCost,
            amount = amount,
            subType = contentSubType
        }
        builder:say(npc, "Do you want to buy " .. amount .. "x " .. itemName ..
            " for " .. (unitCost * amount) .. " gold coins?", player)
        InstanceState.updateTalkStart(npc:getId(), player:getId())
        return true
    end)

    -- Register confirmation handler via centralized dispatcher (once — idempotent)
    if not self._confirmHandlers["container_buy_confirm"] then
        self:registerConfirmation("container_buy_confirm", function(npc, player, builder, s)
            local sel = s.shopSelection
            s.topic = 0
            s.shopSelection = nil

            local totalCost = sel.cost * sel.amount
            local ret = doPlayerBuyItemContainer(
                player:getId(), sel.containerId, sel.itemId,
                sel.amount, totalCost, sel.subType)
            if ret then
                builder:say(npc, "Here you are.", player)
            else
                builder:say(npc, "You don't have enough money.", player)
            end
            InstanceState.updateTalkStart(npc:getId(), player:getId())
            return true
        end, function(npc, player, builder, s)
            s.topic = 0
            s.shopSelection = nil
            builder:say(npc, builder._declineMsg, player)
            InstanceState.updateTalkStart(npc:getId(), player:getId())
            return true
        end)
    end

    return self
end

function ShopBuilder:onEndTrade(callback)
    self._onEndTradeCallback = callback; return self
end


function ShopBuilder:register()
    self._speechBubble = SPEECHBUBBLE_TRADE

    -- Add "trade" keyword to open shop window
    self._keywords:addKeyword({"trade"}, function(npc, player, message, builder)
        if not builder:isFocused(npc:getId(), player:getId()) then return false end
        npc:openShopWindow(player, builder._shopItems,
            function() return true end,
            function() return true end)
        builder:say(npc, builder._tradeMsg, player)
        InstanceState.updateTalkStart(npc:getId(), player:getId())
        return true
    end, 10) -- high priority

    -- Create NpcType and register standard callbacks via parent logic
    local npcType = Game.createNpcType(self._name)
    npcType:name(self._name)
    npcType:health(self._health)
    npcType:maxHealth(self._maxHealth)
    npcType:walkInterval(self._walkInterval)
    npcType:walkRadius(self._walkRadius)
    npcType:baseSpeed(self._baseSpeed)
    npcType:floorChange(self._floorChange)
    npcType:isPushable(self._pushable)
    npcType:outfit(self._outfit)
    npcType:speechBubble(self._speechBubble)

    EventDispatcher.register(self._name, self)
    EventDispatcher.setupCallbacks(npcType, self._name)

    -- Shop-specific: onBuyItem
    local builder = self
    npcType:eventType(NPCS_EVENT_BUYITEM)
    npcType:onBuyItem(function(npc, player, itemId, subType, amount, ignoreCap, inBackpacks)
        return builder:handleBuyItem(npc, player, itemId, subType, amount, ignoreCap, inBackpacks)
    end)

    -- Shop-specific: onSellItem
    npcType:eventType(NPCS_EVENT_SELLITEM)
    npcType:onSellItem(function(npc, player, itemId, subType, amount, ignoreEquipped)
        return builder:handleSellItem(npc, player, itemId, subType, amount, ignoreEquipped)
    end)

    -- Shop-specific: onPlayerEndTrade (only if callback is registered and API exists)
    -- Note: NPCS_EVENT_PLAYER_ENDTRADE is not available in TFS 1.4.2
    -- The onEndTrade callback can be triggered via handleCloseChannel instead

    npcType:register()
    return self
end
