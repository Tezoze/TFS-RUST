-- Blood Brothers Quest - Marziel Statue Position Detection
-- Detects when player steps on the tile directly in front of Vampire Lord Statue

function onStepIn(creature, item, position, fromPosition)
    if not creature or not creature:isPlayer() then
        return true
    end

    local player = creature

    -- Check if player is female
    if player:getSex() ~= PLAYERSEX_FEMALE then
        player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "Only female characters can perform this ritual.")
        return true
    end

    -- Check if player is on Mission 9
    if player:getStorageValue(Storage.BloodBrothers.Mission09) ~= 1 then
        player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "You have no reason to perform this ritual right now.")
        return true
    end

    -- Find the Vampire Lord Statue at position (32940, 31457, 2)
    -- Note: Movement is triggered on tile (32940, 31458, 2) which is in front of the statue
    local statuePosition = Position(32940, 31457, 2)
    local statueTile = Tile(statuePosition)
    local statueItem = statueTile and statueTile:getItemById(9241)

    if statueItem then
        -- Transform statue to vampire lord (9242) for 1 minute
        statueItem:transform(9242)

        -- Schedule reversion back to statue after 1 minute (60000 milliseconds)
        addEvent(function()
            if statueItem and statueItem:isItem() and statueItem:getId() == 9242 then
                statueItem:transform(9241)
            end
        end, 60000)

        -- Display creature message
        player:say("AAAAH... THE SCENT OF A WOMAN... GIVE ME MORE...", TALKTYPE_MONSTER_SAY)
    end

    -- Set storage value indicating player is in position for the ritual
    player:setStorageValue(Storage.BloodBrothers.MarzielStatuePosition, 1)

    return true
end

function onStepOut(creature, item, position, fromPosition)
    if not creature or not creature:isPlayer() then
        return true
    end

    local player = creature

    -- Clear the position storage when player steps away
    player:setStorageValue(Storage.BloodBrothers.MarzielStatuePosition, 0)

    return true
end
