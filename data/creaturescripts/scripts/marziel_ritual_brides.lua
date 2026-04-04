-- Blood Brothers Quest - Marziel Ritual Vampire Bride Death Handler

local MARZIEL_ROOM_POSITION = Position(32975, 31461, 1) -- Position to teleport player after killing brides
local MARZIEL_POSITION = Position(32975, 31459, 1) -- Position for Marziel
local VAMPIRE_POSITION = Position(32976, 31458, 1) -- Position for the Vampire

function onDeath(creature, corpse, killer, mostDamageKiller, lastHitUnjustified, mostDamageUnjustified)
    -- Only process if this is a Vampire Bride
    if creature:getName() ~= "Vampire Bride" then
        return true
    end

    -- Get the killer (should be the player doing the ritual)
    local player = killer
    if not player or not player:isPlayer() then
        return true
    end

    -- Check if player has started the Marziel ritual
    if player:getStorageValue(Storage.BloodBrothers.MarzielRitualStarted) ~= 1 then
        return true
    end

    -- Count how many Vampire Brides are still alive in the area
    local bridesAlive = 0
    local bridePositions = {
        Position(32969, 31460, 1), -- Position for first Vampire Bride
        Position(32971, 31460, 1)  -- Position for second Vampire Bride
    }

    for _, pos in ipairs(bridePositions) do
        local tile = Tile(pos)
        if tile then
            local creatures = tile:getCreatures()
            if creatures then
                for _, tileCreature in ipairs(creatures) do
                    if tileCreature:getName() == "Vampire Bride" then
                        bridesAlive = bridesAlive + 1
                    end
                end
            end
        end
    end

    -- If no brides are alive, teleport player to Marziel's room and spawn Marziel + Vampire
    if bridesAlive == 0 then
        -- Teleport player to Marziel's room
        player:teleportTo(MARZIEL_ROOM_POSITION)
        player:getPosition():sendMagicEffect(CONST_ME_TELEPORT)

        -- Spawn Marziel and a Vampire
        Game.createMonster("Marziel", MARZIEL_POSITION)
        Game.createMonster("Vampire", VAMPIRE_POSITION)

        -- Reset ritual storage
        player:setStorageValue(Storage.BloodBrothers.MarzielRitualStarted, 0)

        player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "You have defeated the Vampire Brides! You have been teleported to Marziel's chamber.")
    end

    return true
end
