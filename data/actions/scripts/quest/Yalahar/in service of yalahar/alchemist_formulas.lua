function onUse(player, item, fromPosition, target, toPosition, isHotkey)
    -- Check if player is at the correct quest stage
    local questline = player:getStorageValue(Storage.InServiceofYalahar.Questline) or 0
    if questline ~= 20 and questline ~= 21 then
        player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "You can't access this right now.")
        return true
    end

    -- Check if AlchemistFormula storage is set to 0 (enabled to collect)
    local formulaStatus = player:getStorageValue(Storage.InServiceofYalahar.AlchemistFormula) or -1
    if formulaStatus ~= 0 then
        if formulaStatus == 1 then
            player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "You have already collected the alchemist formulas.")
        else
            player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "You need to eliminate the plague carriers first.")
        end
        return true
    end

    -- Check if all three diseased creatures have been killed
    local diseasedBill = player:getStorageValue(Storage.InServiceofYalahar.DiseasedBill) or 0
    local diseasedDan = player:getStorageValue(Storage.InServiceofYalahar.DiseasedDan) or 0
    local diseasedFred = player:getStorageValue(Storage.InServiceofYalahar.DiseasedFred) or 0

    if diseasedBill ~= 1 or diseasedDan ~= 1 or diseasedFred ~= 1 then
        player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "You must eliminate all plague carriers before collecting the research formulas.")
        return true
    end

    -- Give the alchemist formulas to the player
    if player:addItem(9733, 1) then
        -- Set storage to indicate formulas have been collected
        player:setStorageValue(Storage.InServiceofYalahar.AlchemistFormula, 1)
        player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "You have found and collected 'The Alchemists' Formulas'. You can now decide whether to give them to Azerus or destroy them as Palimuth suggested.")
        
        -- Add some visual effect
        toPosition:sendMagicEffect(CONST_ME_MAGIC_BLUE)
        player:say("You found the alchemist formulas!", TALKTYPE_MONSTER_SAY)
    else
        player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "You don't have enough space in your inventory.")
    end

    return true
end

