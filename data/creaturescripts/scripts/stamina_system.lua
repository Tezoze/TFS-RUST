--[[
    Stamina System (Traditional CreatureScript)
    - Offline regen: Regenerates stamina based on time logged out
    
    Note: Online regen requires a separate GlobalEvent (see globalevents/scripts/stamina_online_regen.lua)
]]

-- Configuration
local CONFIG = {
    OFFLINE_GRACE_PERIOD = 600,       -- 10 min before offline regen starts
    MIN_OFFLINE_TIME = 180,           -- 3 min minimum offline
    NORMAL_STAMINA_CAP = 2340,        -- 39 hours (normal regen cap)
    MAX_STAMINA = 2520,               -- 42 hours cap
}

function onLogin(player)
    if not configManager.getBoolean(configKeys.STAMINA_SYSTEM) then
        return true
    end
    
    -- Calculate offline time
    local lastLogout = player:getLastLogout()
    local offlineTime = (lastLogout ~= 0) and math.min(os.time() - lastLogout, 86400 * 21) or 0
    offlineTime = offlineTime - CONFIG.OFFLINE_GRACE_PERIOD
    
    if offlineTime < CONFIG.MIN_OFFLINE_TIME then
        return true
    end
    
    local staminaMinutes = player:getStamina()
    local initialStamina = staminaMinutes
    
    local maxNormalStaminaRegen = CONFIG.NORMAL_STAMINA_CAP - math.min(CONFIG.NORMAL_STAMINA_CAP, staminaMinutes)
    local regainStaminaMinutes = offlineTime / configManager.getNumber(configKeys.STAMINA_REGEN_MINUTE)
    
    if regainStaminaMinutes > maxNormalStaminaRegen then
        local happyHourRegen = (offlineTime - (maxNormalStaminaRegen * 180)) / configManager.getNumber(configKeys.STAMINA_REGEN_PREMIUM)
        staminaMinutes = math.min(CONFIG.MAX_STAMINA, math.max(CONFIG.NORMAL_STAMINA_CAP, staminaMinutes) + happyHourRegen)
    else
        staminaMinutes = staminaMinutes + regainStaminaMinutes
    end
    
    player:setStamina(staminaMinutes)
    
    local regained = math.floor(staminaMinutes - initialStamina)
    if regained > 0 then
        player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "You regenerated " .. regained .. " stamina minutes while offline.")
    end
    
    return true
end
