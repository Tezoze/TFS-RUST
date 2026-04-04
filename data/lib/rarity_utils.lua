--[[
    Shared Rarity Utility Functions
    Used by rarity_scroll.lua and reroll_scroll.lua
]]

-- Get the current rarity tier from item description
function getItemRarity(item)
    local desc = item:getAttribute(ITEM_ATTRIBUTE_DESCRIPTION) or ""
    local descLower = desc:lower()

    if descLower:find("mythic") then
        return "mythic"
    elseif descLower:find("legendary") then
        return "legendary"
    elseif descLower:find("epic") then
        return "epic"
    elseif descLower:find("rare") then
        return "rare"
    end
    return nil
end

-- Strip all rarity attributes from an item (reset to base stats)
function stripRarity(item)
    local itemType = ItemType(item:getId())

    -- Reset description to base
    local baseDesc = itemType:getDescription()
    if baseDesc == "" then
        item:removeAttribute(ITEM_ATTRIBUTE_DESCRIPTION)
    else
        item:setAttribute(ITEM_ATTRIBUTE_DESCRIPTION, baseDesc)
    end

    -- Reset all stats to base values
    local baseAttack = itemType:getAttack()
    local baseDefense = itemType:getDefense()
    local baseExtraDefense = itemType:getExtraDefense()
    local baseArmor = itemType:getArmor()
    local baseHitChance = itemType:getHitChance()
    local baseShootRange = itemType:getShootRange()

    if baseAttack > 0 then item:setAttribute(ITEM_ATTRIBUTE_ATTACK, baseAttack) end
    if baseDefense > 0 then item:setAttribute(ITEM_ATTRIBUTE_DEFENSE, baseDefense) end
    if baseExtraDefense > 0 then item:setAttribute(ITEM_ATTRIBUTE_EXTRADEFENSE, baseExtraDefense) end
    if baseArmor > 0 then item:setAttribute(ITEM_ATTRIBUTE_ARMOR, baseArmor) end
    if baseHitChance > 0 then item:setAttribute(ITEM_ATTRIBUTE_HITCHANCE, baseHitChance) end
    if baseShootRange > 0 then item:setAttribute(ITEM_ATTRIBUTE_SHOOTRANGE, baseShootRange) end

    -- Reset container size to default
    if itemType:isContainer() then
        item:removeAttribute(ITEM_ATTRIBUTE_CONTAINERSIZE)
    end
end
