function onKill(creature, target)
    -- 1. Check if the dying monster is Tirecz
    if not target or target:getName():lower() ~= 'tirecz' then
        return true
    end

    local exitPosition = Position(33053, 31022, 7)
    local arenaCenter = Position(33063, 31034, 3)
    
    -- 2. Find everyone in the room (Range: 10x10 squares from center)
    local spectators = Game.getSpectators(arenaCenter, false, false, 10, 10, 10, 10)
    
    -- Collect monsters to remove AFTER processing (avoid modifying during iteration)
    local monstersToRemove = {}
    
    for i = 1, #spectators do
        local spectator = spectators[i]
        
        if spectator:isPlayer() then
            -- TEAM LOGIC: Handle Winner
            spectator:teleportTo(exitPosition)
            spectator:getPosition():sendMagicEffect(CONST_ME_TELEPORT)
            spectator:sendTextMessage(MESSAGE_EVENT_ADVANCE, 'You have won! Take your reward from the chest.')
            
            -- Quest Progress Update
            if spectator:getStorageValue(Storage.TheNewFrontier.Mission09) == 1 then
                spectator:setStorageValue(Storage.TheNewFrontier.Mission09, 2)
                spectator:setStorageValue(Storage.TheNewFrontier.Questline, 26)
                spectator:sendTextMessage(MESSAGE_EVENT_ADVANCE, 'You have defeated Tirecz!')
            end
            
        else
            -- CLEANUP LOGIC: Collect monsters for removal (not Tirecz)
            if spectator:getId() ~= target:getId() then
                table.insert(monstersToRemove, spectator:getId())
            end
        end
    end

    -- 3. Visuals at the exit
    exitPosition:sendMagicEffect(CONST_ME_TELEPORT)

    -- 4. UNLOCK THE ARENA
    Game.setStorageValue(Storage.TheNewFrontier.Mission09, -1)
    
    -- 5. Remove monsters safely AFTER all processing is done
    -- Use addEvent to defer removal to next server tick, avoiding mid-iteration crashes
    addEvent(function(monsterIds)
        for _, monsterId in ipairs(monsterIds) do
            local monster = Creature(monsterId)
            if monster and not monster:isPlayer() then
                monster:remove()
            end
        end
    end, 100, monstersToRemove)
    
    return true
end