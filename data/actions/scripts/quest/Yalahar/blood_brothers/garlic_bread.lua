function onUse(player, item, fromPosition, target, toPosition, isHotkey)
    -- Flour (2692) + Holy Water → Holy Water Dough (9112)
    if item.itemid == 2692 then
        if target.itemid == 7494 and target.type == 1 then -- Holy water in a liquid container
            item:remove(1)
            player:addItem(9112, 1) -- Lump of holy water dough
            target:transform(2006) -- Transform to empty vial
            player:getPosition():sendMagicEffect(CONST_ME_MAGIC_BLUE)
            return true
        end
    end

    -- Holy Water Dough (9112) + Bulb of Garlic (9114) → Garlic Dough (9113)
    if item.itemid == 9112 then
        if target.itemid == 9114 then -- Bulb of garlic
            item:remove(1)
            target:remove(1)
            player:addItem(9113, 1) -- Lump of garlic dough
            player:getPosition():sendMagicEffect(CONST_ME_MAGIC_BLUE)
            return true
        end
    end

    -- Garlic Dough (9113) + Baking Tray (2561) → Baking Tray with Garlic Dough (9115)
    if item.itemid == 9113 then
        if target.itemid == 2561 then -- Empty baking tray
            item:remove(1)
            target:transform(9115) -- Baking tray with garlic cookie dough
            player:getPosition():sendMagicEffect(CONST_ME_MAGIC_BLUE)
            return true
        end
    end

    return false
end
