-- Blood Brothers Quest - Castle Re-entry
-- Allows players who completed the ritual to re-enter the castle

function onStepIn(creature, item, position, fromPosition)
    local player = creature:getPlayer()
    if not player then
        return true
    end

    -- Check if player has completed the ritual
    local ritualCompleted = player:getStorageValue(Storage.BloodBrothers.BloodCrystal.RitualCompleted) == 1

    if not ritualCompleted then
        -- Silently do nothing if requirements not met
        return true
    end

    -- Allow re-entry to castle
    local CASTLE_ENTRANCE = Position(32953, 31483, 6) -- Castle entrance interior
    player:teleportTo(CASTLE_ENTRANCE)
    player:getPosition():sendMagicEffect(CONST_ME_TELEPORT)

    return true
end
