--[[
    Online Stamina Regen (Traditional GlobalEvent)
    Regenerates stamina while in-game if not in combat
]]

-- Configuration
local CONFIG = {
    STAMINA_PER_TICK = 1,             -- Minutes per tick while online
    MAX_STAMINA = 2520,               -- 42 hours cap
}

function onThink(interval)
    if not configManager.getBoolean(configKeys.STAMINA_SYSTEM) then
        return true
    end
    
    for _, player in ipairs(Game.getPlayers()) do
        -- Only regen if player is NOT in combat (no infight condition)
        if not player:hasCondition(CONDITION_INFIGHT) then
            local stamina = player:getStamina()
            if stamina < CONFIG.MAX_STAMINA then
                player:setStamina(math.min(CONFIG.MAX_STAMINA, stamina + CONFIG.STAMINA_PER_TICK))
            end
        end
    end
    
    return true
end
