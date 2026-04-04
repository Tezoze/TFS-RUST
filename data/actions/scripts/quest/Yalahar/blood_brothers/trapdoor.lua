-- Blood Brothers Quest - Item Transformer
-- Transforms item 9629 into 9625, then reverts back after 1 minute

local TRANSFORM_ITEM_ID = 9629
local TRANSFORMED_ITEM_ID = 9625
local REVERT_TIME = 60000 -- 1 minute in milliseconds

function onUse(player, item, fromPosition, target, toPosition, isHotkey)
    if item:getId() ~= TRANSFORM_ITEM_ID then
        return false
    end

    -- Transform the item
    item:transform(TRANSFORMED_ITEM_ID)

    -- Schedule revert after 1 minute
    addEvent(function()
        if item then
            item:transform(TRANSFORM_ITEM_ID)
        end
    end, REVERT_TIME)

    return true
end
