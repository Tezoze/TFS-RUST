-- Blood Brothers Quest - Marziel Ritual Trigger
-- Triggers when blood pool appears on the ritual tile

local MARZIEL_ROOM_POSITION = Position(32940, 31461, 1) -- Position to teleport players to Marziel's chamber
local RITUAL_POSITION = Position(32940, 31458, 2) -- The ritual tile position

function onAddItem(moveitem, tileitem, position, cid)
    -- Check if blood pool was added at the ritual position
    if position.x ~= RITUAL_POSITION.x or position.y ~= RITUAL_POSITION.y or position.z ~= RITUAL_POSITION.z then
        return true
    end
    
    -- Find the player who created the blood pool (should be nearby)
    local tile = Tile(position)
    if not tile then
        return true
    end
    
    -- Check surrounding tiles for players
    local player = nil
    for x = position.x - 1, position.x + 1 do
        for y = position.y - 1, position.y + 1 do
            local checkTile = Tile(Position(x, y, position.z))
            if checkTile then
                local creatures = checkTile:getCreatures()
                if creatures then
                    for _, creature in ipairs(creatures) do
                        if creature:isPlayer() then
                            player = creature
                            break
                        end
                    end
                end
            end
            if player then break end
        end
        if player then break end
    end
    
    if not player then
        return true
    end
    
    -- Check if player is female
    if player:getSex() ~= PLAYERSEX_FEMALE then
        player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "Only female characters can perform this ritual.")
        return true
    end
    
    -- Check if player is on Mission 9 or has completed it (Mission 9 storage >= 1)
    local mission09Value = player:getStorageValue(Storage.BloodBrothers.Mission09)
    if mission09Value < 1 then
        player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "You have no reason to perform this ritual right now.")
        return true
    end
    
    -- Check if ritual was recently performed (cooldown to prevent multiple triggers from blood decay)
    if player:getStorageValue(Storage.BloodBrothers.MarzielRitualCooldown) > os.time() then
        player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "The ritual energy is still settling. Wait a moment before trying again.")
        return true
    end
    
    -- Set cooldown (30 seconds to prevent spam but allow helping multiple friends)
    player:setStorageValue(Storage.BloodBrothers.MarzielRitualCooldown, os.time() + 30)
    
    -- Send effect at the ritual position
    position:sendMagicEffect(CONST_ME_DRAWBLOOD)
    
    -- Remove the blood pool to prevent decay triggers
    moveitem:remove()
    
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
            local checkTile = Tile(tilePos)
            if checkTile then
                local creatures = checkTile:getCreatures()
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
    end
    
    -- Spawn Marziel and 2 Vampires in the chamber
    Game.createMonster("Marziel", Position(32940, 31459, 1))
    Game.createMonster("Vampire", Position(32939, 31458, 1))
    Game.createMonster("Vampire", Position(32941, 31458, 1))
    
    return true
end

