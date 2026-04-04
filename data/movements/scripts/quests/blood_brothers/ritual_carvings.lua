-- Blood Brothers Quest - Strange Carvings Ritual
-- Coordinates 4 players with charged blood crystals to teleport deeper into castle
-- Uses position checking to identify which of the 4 carvings the player is on

local TELEPORT_DESTINATION = Position(32953, 31480, 6) -- UPDATE: Deeper castle location

local RITUAL_STORAGE = {
    ACTIVE_PLAYERS = 46000,  -- Bitmask of active positions (1, 2, 4, 8)
    COMPLETED = 46001        -- Whether ritual was completed
}

-- Define the 4 carving positions - UPDATE THESE COORDINATES
local CARVING_POSITIONS = {
    Position(32928, 31460, 7), -- Carving 1 - UPDATE COORDINATES
    Position(32946, 31499, 6), -- Carving 2 - UPDATE COORDINATES
    Position(32966, 31498, 6), -- Carving 3 - UPDATE COORDINATES
    Position(32941, 31510, 7)  -- Carving 4 - UPDATE COORDINATES
}

function onStepIn(creature, item, position, fromPosition)
    local player = creature:getPlayer()
    if not player then
        return true
    end

    local cid = player:getId()

    -- Check if player has charged blood crystal
    if player:getItemCount(9141) == 0 or player:getStorageValue(Storage.BloodBrothers.BloodCrystal.Charged) ~= 1 then
        return true
    end

    -- Check if player is on Mission 5
    if player:getStorageValue(Storage.BloodBrothers.Mission05) < 1 then
        return true
    end

    -- Find which carving position this is
    local positionIndex = nil
    for i, carvingPos in ipairs(CARVING_POSITIONS) do
        if position.x == carvingPos.x and position.y == carvingPos.y and position.z == carvingPos.z then
            positionIndex = i
            break
        end
    end

    if not positionIndex then
        return true -- Not a ritual carving
    end

    -- Check if ritual already completed
    local ritualCompleted = Game.getStorageValue(RITUAL_STORAGE.COMPLETED) or 0
    if ritualCompleted == 1 then
        return true
    end

    -- Mark this position as occupied
    local activeMask = Game.getStorageValue(RITUAL_STORAGE.ACTIVE_PLAYERS) or 0
    local positionBit = bit.lshift(1, positionIndex - 1)

    if bit.band(activeMask, positionBit) ~= 0 then
        return true
    end

    -- Occupy this position
    activeMask = bit.bor(activeMask, positionBit)
    Game.setStorageValue(RITUAL_STORAGE.ACTIVE_PLAYERS, activeMask)
    Game.setStorageValue(46010 + positionIndex, cid)

    -- Check if all 4 positions are occupied
    if activeMask == 15 then -- Binary 1111 = all 4 positions
        -- Complete the ritual!
        Game.setStorageValue(RITUAL_STORAGE.COMPLETED, 1)

        -- Teleport all 4 players and remove their blood crystals
        for i = 1, 4 do
            local playerId = Game.getStorageValue(46010 + i)
            if playerId and playerId > 0 then
                local ritualPlayer = Player(playerId)
                if ritualPlayer then
                    -- Remove the charged blood crystal from player
                    ritualPlayer:removeItem(9141, 1) -- Charged Blood Crystal
                    ritualPlayer:setStorageValue(Storage.BloodBrothers.BloodCrystal.RitualCompleted, 1)
                    ritualPlayer:setStorageValue(Storage.BloodBrothers.Mission05, 4) -- Ritual completed, return to Julius
                    ritualPlayer:teleportTo(TELEPORT_DESTINATION)
                    ritualPlayer:getPosition():sendMagicEffect(CONST_ME_TELEPORT)
                end
            end
        end

        -- Reset ritual storage
        Game.setStorageValue(RITUAL_STORAGE.ACTIVE_PLAYERS, 0)
        Game.setStorageValue(RITUAL_STORAGE.COMPLETED, 0)
        for i = 1, 4 do
            Game.setStorageValue(46010 + i, 0)
        end
    end

    return true
end

function onStepOut(creature, item, position, fromPosition)
    local player = creature:getPlayer()
    if not player then
        return true
    end

    -- Find which carving position this was
    local positionIndex = nil
    for i, carvingPos in ipairs(CARVING_POSITIONS) do
        if position.x == carvingPos.x and position.y == carvingPos.y and position.z == carvingPos.z then
            positionIndex = i
            break
        end
    end

    if not positionIndex then
        return true -- Not a ritual carving
    end

    -- Check if ritual already completed
    local ritualCompleted = Game.getStorageValue(RITUAL_STORAGE.COMPLETED) or 0
    if ritualCompleted == 1 then
        return true
    end

    -- Clear this position
    local activeMask = Game.getStorageValue(RITUAL_STORAGE.ACTIVE_PLAYERS) or 0
    local positionBit = bit.lshift(1, positionIndex - 1)

    activeMask = bit.band(activeMask, bit.bnot(positionBit))
    Game.setStorageValue(RITUAL_STORAGE.ACTIVE_PLAYERS, activeMask)
    Game.setStorageValue(46010 + positionIndex, 0)

    return true
end
