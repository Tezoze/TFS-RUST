-- Blood Brothers Quest - Broken Mirrors Action
-- Break 2 mirrors to access Lersatio's chamber

local LERSATIO_ROOM_POSITION = Position(32967, 31461, 1) -- UPDATE: Lersatio's room position
-- Storage to track broken mirrors per player

function onUse(player, item, fromPosition, target, toPosition, isHotkey)
    if not item or not item:isItem() then
        return false
    end

    local itemId = item:getId()

    -- Check if item is a wall mirror
    if itemId ~= 9583 and itemId ~= 9584 and itemId ~= 9585 and itemId ~= 9586 and itemId ~= 9587 then
        return false
    end

    -- Check if player is on Mission 8 (Lersatio) or higher
    if player:getStorageValue(Storage.BloodBrothers.Mission08) < 1 then
        player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "You have no reason to break this mirror right now.")
        return true
    end


    -- Get current broken mirror count for this player
    local brokenCount = player:getStorageValue(Storage.BloodBrothers.BrokenMirrors)
    if brokenCount == -1 then
        brokenCount = 0
    end

    -- Break the mirror - transform to cracked mirror (9637) for 1 minute
    item:transform(9637) -- Transform to cracked wall mirror
    player:getPosition():sendMagicEffect(CONST_ME_BLOCKHIT)

    -- Schedule reversion after 1 minute (60000 milliseconds)
    addEvent(function()
        if item and item:isItem() and item:getId() == 9637 then
            item:transform(9583) -- Revert back to original mirror
        end
    end, 60000)

    -- Increment broken mirror count
    brokenCount = brokenCount + 1
    player:setStorageValue(Storage.BloodBrothers.BrokenMirrors, brokenCount)

    if brokenCount < 2 then
        player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "You broke one mirror. You need to break one more to access Lersatio's chamber.")
    else
        -- Broken 2 mirrors - teleport to Lersatio's room
        player:teleportTo(LERSATIO_ROOM_POSITION)
        player:getPosition():sendMagicEffect(CONST_ME_TELEPORT)

        -- Spawn Lersatio and 2 Vampires
        Game.createMonster("Lersatio", Position(32967, 31459, 1))
        Game.createMonster("Vampire", Position(32966, 31458, 1))
        Game.createMonster("Vampire", Position(32968, 31458, 1))

        -- Reset mirror count
        player:setStorageValue(Storage.BloodBrothers.BrokenMirrors, 0)

        player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "The mirrors are broken! You have been teleported to Lersatio's chamber.")
    end

    return true
end
