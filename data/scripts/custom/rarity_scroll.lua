--[[
    Rarity Scroll Upgrade System (Revscript)
    
    Scrolls upgrade items through rarity tiers. Failure only consumes the scroll:
    - Rare Scroll (18419): Normal -> Rare (70% success)
    - Epic Scroll (18414): Rare -> Epic (55% success)
    - Legendary Scroll (24115): Epic -> Legendary (30% success)
    - Mythic Scroll (18420): Legendary -> Mythic (15% success)
]]

local scrollData = {
    [18419] = {name = "Rare",      targetTier = "rare",      requiredTier = nil,         downgradeTo = nil,     successChance = 70},
    [18414] = {name = "Epic",      targetTier = "epic",      requiredTier = "rare",      downgradeTo = nil,     successChance = 55},
    [24115] = {name = "Legendary", targetTier = "legendary", requiredTier = "epic",      successChance = 30},
    [18420] = {name = "Mythic",    targetTier = "mythic",    requiredTier = "legendary", successChance = 15},
}

local rarityScroll = Action()

function rarityScroll.onUse(player, item, fromPosition, target, toPosition, isHotkey)
    local info = scrollData[item:getId()]
    if not info then
        return false
    end

    if not target or not target.itemid then
        player:sendCancelMessage("You can only use this on equipment.")
        return true
    end

    local tile = Tile(toPosition)
    if tile and tile:getGround() and target.itemid == tile:getGround():getId() then
        player:sendCancelMessage("You cannot use this on the ground.")
        return true
    end

    local itemType = ItemType(target.itemid)

    if itemType:isStackable() then
        player:sendCancelMessage("You can only use this on equipment, not consumables.")
        return true
    end

    local targetItem = nil
    if target.uid and target.uid > 0 then
        targetItem = Item(target.uid)
    else
        if tile then
            targetItem = tile:getTopVisibleThing(player)
        end
    end

    if not targetItem or not targetItem:isItem() or targetItem:getId() ~= target.itemid then
        player:sendCancelMessage("Unable to find the target item.")
        return true
    end

    if rollCheck and not rollCheck(targetItem) then
        player:sendCancelMessage("This item cannot be enhanced with rarity.")
        return true
    end

    local currentRarity = getItemRarity(targetItem)
    
    if info.requiredTier then
        if currentRarity ~= info.requiredTier then
            local requiredName = info.requiredTier:gsub("^%l", string.upper)
            player:sendCancelMessage("This scroll can only be used on " .. requiredName .. " items.")
            return true
        end
    else
        if currentRarity ~= nil then
            player:sendCancelMessage("This scroll can only be used on normal (non-rare) items.")
            return true
        end
    end

    item:remove(1)
    
    local roll = math.random(1, 100)
    
    if roll <= info.successChance then
        stripRarity(targetItem)
        local rollResult = rollRarity(targetItem, info.targetTier)
        
        if rollResult > 0 then
            player:sendTextMessage(MESSAGE_INFO_DESCR, "Success! Your item has been upgraded to " .. info.name .. " rarity!")
            toPosition:sendMagicEffect(CONST_ME_MAGIC_GREEN)
        else
            player:sendTextMessage(MESSAGE_INFO_DESCR, "The upgrade failed unexpectedly. The scroll was consumed.")
            toPosition:sendMagicEffect(CONST_ME_POFF)
        end
    else
        player:sendTextMessage(MESSAGE_INFO_DESCR, "The upgrade failed, but your item remains unchanged.")
        toPosition:sendMagicEffect(CONST_ME_POFF)
    end

    return true
end

-- Register all scroll item IDs
rarityScroll:id(18419, 18414, 24115, 18420)
rarityScroll:register()
