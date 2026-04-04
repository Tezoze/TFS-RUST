function onLogin(player)
    local lastLogout = player:getLastLogout()
    local offlineTime = lastLogout ~= 0 and math.min(os.time() - lastLogout, 86400 * 21) or 0
    local offlineTrainingSkill = player:getOfflineTrainingSkill()
    if offlineTrainingSkill == -1 then
        player:addOfflineTrainingTime(offlineTime * 1000)
        return true
    end

    player:setOfflineTrainingSkill(-1)

    if offlineTime < 600 then
        player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "You must be logged out for more than 10 minutes to start offline training.")
        return true
    end

    -- Soul point regeneration during offline training (only for mages and paladins)
    -- Regenerate 1 soul point per 2 minutes of training (same rate as hunting)
    local vocation = player:getVocation()
    local vocationId = vocation:getId()
    local soulGained = 0
    
    -- Only regenerate soul for non-knights (vocation 1,2,3,5,6,7 = mages and paladins)
    if vocationId ~= 4 and vocationId ~= 8 then
        local maxSoul = vocation:getMaxSoul()
        local currentSoul = player:getSoul()
        if currentSoul < maxSoul then
            soulGained = math.min(math.floor(offlineTime / 120), maxSoul - currentSoul) -- 1 soul per 2 minutes
            if soulGained > 0 then
                player:addSoul(soulGained)
            end
        end
    end

    -- Validate that the selected skill is compatible with the player's vocation

    if (vocationId == 1 or vocationId == 2 or vocationId == 5 or vocationId == 6) then
        if offlineTrainingSkill ~= SKILL_MAGLEVEL then
            player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "Mages can only train magic level offline.")
            return true
        end
    elseif (vocationId == 3 or vocationId == 7) then
        if offlineTrainingSkill ~= SKILL_DISTANCE then
            player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "Paladins can only train distance fighting offline.")
            return true
        end
    elseif (vocationId == 4 or vocationId == 8) then
        if offlineTrainingSkill == SKILL_MAGLEVEL then
            player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "Knights cannot train magic level offline.")
            return true
        elseif not table.contains({SKILL_SWORD, SKILL_DISTANCE}, offlineTrainingSkill) then
            player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "Knights can only train melee or distance skills offline.")
            return true
        end
    end

    local trainingTime = math.max(0, math.min(offlineTime, math.min(43200, player:getOfflineTrainingTime() / 1000)))
    player:removeOfflineTrainingTime(trainingTime * 1000)

    local remainder = offlineTime - trainingTime
    if remainder > 0 then
        player:addOfflineTrainingTime(remainder * 1000)
    end

    if trainingTime < 60 then
        return true
    end

    local text = "During your absence you trained for"
    local hours = math.floor(trainingTime / 3600)
    if hours > 1 then
        text = string.format("%s %d hours", text, hours)
    elseif hours == 1 then
        text = string.format("%s 1 hour", text)
    end

    local minutes = math.floor((trainingTime % 3600) / 60)
    if minutes ~= 0 then
        if hours ~= 0 then
            text = string.format("%s and", text)
        end

        if minutes > 1 then
            text = string.format("%s %d minutes", text, minutes)
        else
            text = string.format("%s 1 minute", text)
        end
    end

    text = string.format("%s.", text)
    if soulGained > 0 then
        text = string.format("%s You also regenerated %d soul point%s.", text, soulGained, soulGained > 1 and "s" or "")
    end
    player:sendTextMessage(MESSAGE_EVENT_ADVANCE, text)

    local promotion = vocation:getPromotion()
    local topVocation = not promotion and vocation or promotion

    local updateSkills = false
    if table.contains({SKILL_SWORD, SKILL_DISTANCE}, offlineTrainingSkill) then
        local modifier = topVocation:getAttackSpeed() / 1000
        -- REMOVED: Multiplier line was here
        local skillTries = (trainingTime / modifier) / (offlineTrainingSkill == SKILL_DISTANCE and 4 or 2)
        updateSkills = player:addOfflineTrainingTries(offlineTrainingSkill, skillTries)
    elseif offlineTrainingSkill == SKILL_MAGLEVEL then
        local gainTicks = topVocation:getManaGainTicks() * 2
        if gainTicks == 0 then
            gainTicks = 1
        end

        -- REMOVED: Multiplier line was here
        local magicTries = trainingTime * (vocation:getManaGainAmount() / gainTicks)
        updateSkills = player:addOfflineTrainingTries(SKILL_MAGLEVEL, magicTries)
    end

    if updateSkills then
        -- REMOVED: Multiplier line was here
        local shieldTries = trainingTime / 4
        player:addOfflineTrainingTries(SKILL_SHIELD, shieldTries)
    end

    return true
end