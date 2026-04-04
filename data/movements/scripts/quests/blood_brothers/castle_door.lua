-- Blood Brothers Quest - Castle Door
-- Mission 6: A Black History - Door accessible after getting Arthei's diary

function onStepIn(creature, item, position, fromPosition)
    local player = creature:getPlayer()
    if not player then
        return true
    end

    -- Check if player has gotten the diary from the bookcase (Mission06 >= 2)
    local mission06Progress = player:getStorageValue(Storage.BloodBrothers.Mission06) or -1

    if mission06Progress >= 2 then
        -- Allow passage through the door
        return true
    else
        -- Block access and send message
        player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "The door is sealed to unwanted intruders.")
        player:teleportTo(fromPosition, true) -- Teleport back to previous position
        return false
    end
end
