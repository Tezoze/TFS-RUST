--[[
    Reroll Scroll System
    
    Rerolls the attributes of an item while keeping its current rarity tier.
    The item keeps the same rarity (Rare/Epic/Legendary/Mythic) but gets new random stats.
    
    Item ID: 18413 (Small Enchanted Amethyst) - can be changed
]]

local REROLL_SCROLL_ID = 18413

local rerollScroll = Action()

function rerollScroll.onUse(player, item, fromPosition, target, toPosition, isHotkey)
    -- Validate scroll
    if item:getId() ~= REROLL_SCROLL_ID then
        return false
    end

    -- Validate target
    if not target or not target.itemid then
        player:sendCancelMessage("You can only use this on equipment.")
        return true
    end

    -- Check if targeting ground
    local tile = Tile(toPosition)
    if tile and tile:getGround() and target.itemid == tile:getGround():getId() then
        player:sendCancelMessage("You cannot use this on the ground.")
        return true
    end

    local itemType = ItemType(target.itemid)

    -- Check if stackable (consumables)
    if itemType:isStackable() then
        player:sendCancelMessage("You can only use this on equipment, not consumables.")
        return true
    end

    -- Get the actual item object
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

    -- Check if item can be enhanced (uses global rollCheck if defined)
    if rollCheck and not rollCheck(targetItem) then
        player:sendCancelMessage("This item cannot be rerolled.")
        return true
    end

    -- Get current rarity
    local currentRarity = getItemRarity(targetItem)
    
    if not currentRarity then
        player:sendCancelMessage("This scroll can only be used on items with rarity (Rare, Epic, Legendary, or Mythic).")
        return true
    end

    -- Consume the scroll
    item:remove(1)
    
    -- Strip current rarity stats
    stripRarity(targetItem)
    
    -- Reroll with the same rarity tier
    rollRarity(targetItem, currentRarity)
    
    local rarityName = currentRarity:gsub("^%l", string.upper)
    player:sendTextMessage(MESSAGE_INFO_DESCR, "Your " .. rarityName .. " item has been rerolled with new attributes!")
    toPosition:sendMagicEffect(CONST_ME_MAGIC_BLUE)

    return true
end

rerollScroll:id(REROLL_SCROLL_ID)
rerollScroll:register()
