-- Blood Brothers Quest - Marziel Ritual
-- Mission 9: Use Vial of Blood ON the ritual tile (UniqueID 45309)

local MARZIEL_ROOM_POSITION = Position(32975, 31461, 1) -- Position to teleport players to Marziel's chamber

function onUse(player, item, fromPosition, target, toPosition, isHotkey)
    print("[DEBUG] Marziel Ritual - Script triggered!")
    print("[DEBUG] Player: " .. player:getName())
    print("[DEBUG] Item used: ID=" .. item:getId() .. ", SubType=" .. item:getSubType())
    print("[DEBUG] Target position: " .. toPosition.x .. ", " .. toPosition.y .. ", " .. toPosition.z)
    
    -- This script is triggered when using the tile with UID 45309
    -- We need to check if the player is using a vial with blood ON this tile
    
    -- The 'item' parameter here is actually the tile being used
    -- We need to check what the player is holding/using
    
    -- Check if a vial with blood exists on the tile (was just poured)
    local tile = Tile(toPosition)
    if not tile then
        print("[DEBUG] No tile found")
        return false
    end
    
    -- Check if there's a pool of blood on the tile
    local bloodPool = tile:getItemById(2016) or tile:getItemById(2017) or tile:getItemById(2018)
    if not bloodPool then
        print("[DEBUG] No blood pool found on tile")
        player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "You must pour blood on this tile to perform the ritual.")
        return true
    end
    
    print("[DEBUG] Blood pool found on ritual tile!")

    -- Check if player is female
    local playerSex = player:getSex()
    print("[DEBUG] Player sex: " .. playerSex .. " (PLAYERSEX_FEMALE = " .. PLAYERSEX_FEMALE .. ")")
    if playerSex ~= PLAYERSEX_FEMALE then
        player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "Only female characters can perform this ritual.")
        return true
    end
    
    print("[DEBUG] Female check passed!")

    -- Check if player is on Mission 9
    local mission09Value = player:getStorageValue(Storage.BloodBrothers.Mission09)
    print("[DEBUG] Mission 9 storage value: " .. mission09Value)
    if mission09Value ~= 1 then
        player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "You have no reason to perform this ritual right now.")
        return true
    end
    
    print("[DEBUG] Mission 9 check passed!")
    print("[DEBUG] All checks passed! Performing ritual...")

    -- Remove the vial
    item:remove()

    -- Send effect at the ritual position
    toPosition:sendMagicEffect(CONST_ME_DRAWBLOOD)

    -- Define the ritual room area (5x5 area around the statue)
    local statuePosition = Position(32940, 31457, 2)
    local roomArea = {
        fromPos = Position(statuePosition.x - 2, statuePosition.y - 2, statuePosition.z),
        toPos = Position(statuePosition.x + 2, statuePosition.y + 2, statuePosition.z)
    }

    -- Find all players in the ritual room
    local playersInRoom = {}
    for x = roomArea.fromPos.x, roomArea.toPos.x do
        for y = roomArea.fromPos.y, roomArea.toPos.y do
            local tilePos = Position(x, y, roomArea.fromPos.z)
            local tile = Tile(tilePos)
            if tile then
                local creatures = tile:getCreatures()
                if creatures then
                    for _, creature in ipairs(creatures) do
                        if creature:isPlayer() then
                            table.insert(playersInRoom, creature)
                        end
                    end
                end
            end
        end
    end

    -- Teleport all players in the room to Marziel's chamber
    for _, roomPlayer in ipairs(playersInRoom) do
        roomPlayer:teleportTo(MARZIEL_ROOM_POSITION)
        roomPlayer:getPosition():sendMagicEffect(CONST_ME_TELEPORT)
        roomPlayer:sendTextMessage(MESSAGE_EVENT_ADVANCE, "The ritual has been performed! You have been teleported to Marziel's chamber.")
    end

    -- Spawn Marziel and a Vampire in the chamber
    Game.createMonster("Marziel", Position(32975, 31459, 1))
    Game.createMonster("Vampire", Position(32976, 31458, 1))

    return true
end
