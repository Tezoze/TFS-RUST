--[[
    Stamina System (Revscript)
    - Offline regen: Regenerates stamina based on time logged out
    - Online regen: Regenerates stamina while in-game if not in combat
]]

-- Configuration
local CONFIG = {
    -- Regen rates (same for online and offline)
    REGEN_INTERVAL = 120 * 1000,      -- Check every 120 seconds (2 min)
    STAMINA_PER_TICK = 1,             -- 1 stamina min per tick (2 min real = 1 stamina min)
    MAX_STAMINA = 2520,               -- 42 hours cap
    NORMAL_STAMINA_CAP = 2400,        -- 40 hours (normal regen cap)
    
    -- Online regen
    OUT_OF_COMBAT_TIME = 300,         -- 5 minutes out of combat before regen starts
    
    -- Offline regen
    OFFLINE_GRACE_PERIOD = 600,       -- 10 min before offline regen starts
    MIN_OFFLINE_TIME = 180,           -- 3 min minimum offline
    OFFLINE_REGEN_RATE = 120,         -- 2 min offline = 1 stamina min (normal)
    OFFLINE_REGEN_PREMIUM = 240,      -- 4 min offline = 1 stamina min (happy hour)
    
    -- Debug
    DEBUG = false,                     -- Set to false to disable debug messages
}

-- Storage keys
local STORAGE_HAPPY_HOUR_TICK = 89999
local STORAGE_LAST_COMBAT_TIME = 89998

local function debugLog(message)
    if CONFIG.DEBUG then
        print("[StaminaSystem] " .. os.date("%H:%M:%S") .. " - " .. message)
    end
end

-- Track when script was loaded and last tick time
local scriptLoadTime = os.time()
local lastTickTime = 0
print("[StaminaSystem] Script loaded at " .. os.date("%H:%M:%S") .. " - Interval: " .. (CONFIG.REGEN_INTERVAL / 1000) .. "s")

-------------------------------------------------
-- OFFLINE REGEN (on login)
-------------------------------------------------
local offlineRegen = CreatureEvent("StaminaOfflineRegen")

function offlineRegen.onLogin(player)
    if not configManager.getBoolean(configKeys.STAMINA_SYSTEM) then
        debugLog("Stamina system disabled in config")
        return true
    end
    
    debugLog(player:getName() .. " logged in - checking offline regen")
    
    -- Initialize combat timer (assume just logged in = fresh, set to now so they wait 5 min)
    player:setStorageValue(STORAGE_LAST_COMBAT_TIME, os.time())
    player:setStorageValue(STORAGE_HAPPY_HOUR_TICK, 0)
    
    -- Calculate offline time
    local lastLogout = player:getLastLogout()
    local offlineTime = (lastLogout ~= 0) and math.min(os.time() - lastLogout, 86400 * 21) or 0
    offlineTime = offlineTime - CONFIG.OFFLINE_GRACE_PERIOD
    
    debugLog(player:getName() .. " was offline for " .. math.floor(offlineTime / 60) .. " minutes (after grace period)")
    
    if offlineTime < CONFIG.MIN_OFFLINE_TIME then
        debugLog(player:getName() .. " - not enough offline time, skipping regen")
        return true
    end
    
    local staminaMinutes = player:getStamina()
    local initialStamina = staminaMinutes
    
    local maxNormalStaminaRegen = CONFIG.NORMAL_STAMINA_CAP - math.min(CONFIG.NORMAL_STAMINA_CAP, staminaMinutes)
    local regainStaminaMinutes = offlineTime / CONFIG.OFFLINE_REGEN_RATE
    
    if regainStaminaMinutes > maxNormalStaminaRegen then
        local happyHourRegen = (offlineTime - (maxNormalStaminaRegen * CONFIG.OFFLINE_REGEN_RATE)) / CONFIG.OFFLINE_REGEN_PREMIUM
        staminaMinutes = math.min(CONFIG.MAX_STAMINA, math.max(CONFIG.NORMAL_STAMINA_CAP, staminaMinutes) + happyHourRegen)
    else
        staminaMinutes = staminaMinutes + regainStaminaMinutes
    end
    
    player:setStamina(staminaMinutes)
    
    local regained = math.floor(staminaMinutes - initialStamina)
    debugLog(player:getName() .. " - offline regen: " .. initialStamina .. " -> " .. math.floor(staminaMinutes) .. " (+" .. regained .. " minutes)")
    
    if regained > 0 then
        player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "You regenerated " .. regained .. " stamina minutes while offline.")
    end
    
    return true
end

offlineRegen:register()

-------------------------------------------------
-- ONLINE REGEN (GlobalEvent timer)
-------------------------------------------------
local onlineRegen = GlobalEvent("StaminaOnlineRegen")

function onlineRegen.onThink(interval)
    local now = os.time()
    local timeSinceLastTick = now - lastTickTime
    lastTickTime = now
    
    debugLog("=== ONLINE REGEN TICK ===")
    debugLog("Interval param: " .. interval .. "ms, Actual time since last: " .. timeSinceLastTick .. "s")
    debugLog("Expected interval: " .. (CONFIG.REGEN_INTERVAL / 1000) .. "s")
    
    if not configManager.getBoolean(configKeys.STAMINA_SYSTEM) then
        debugLog("Stamina system disabled!")
        return true
    end
    
    local players = Game.getPlayers()
    local playerCount = #players
    local regenCount = 0
    local combatCount = 0
    local maxedCount = 0
    local happyHourSkip = 0
    
    for _, player in ipairs(players) do
        local stamina = player:getStamina()
        local inCombat = player:hasCondition(CONDITION_INFIGHT)
        local isHappyHour = stamina >= CONFIG.NORMAL_STAMINA_CAP
        local now = os.time()
        
        -- Track combat time using storage
        local lastCombatTime = player:getStorageValue(STORAGE_LAST_COMBAT_TIME)
        if lastCombatTime < 0 then lastCombatTime = 0 end
        
        -- Update last combat time if in combat
        if inCombat then
            player:setStorageValue(STORAGE_LAST_COMBAT_TIME, now)
            lastCombatTime = now
        end
        
        local timeSinceCombat = now - lastCombatTime
        local canRegen = not inCombat and timeSinceCombat >= CONFIG.OUT_OF_COMBAT_TIME
        
        debugLog(player:getName() .. ": stamina=" .. stamina .. "/" .. CONFIG.MAX_STAMINA .. ", inCombat=" .. tostring(inCombat) .. ", happyHour=" .. tostring(isHappyHour) .. ", timeSinceCombat=" .. timeSinceCombat .. "s")
        
        -- Only regen if player is NOT in combat AND has been out of combat for 5 minutes
        if inCombat then
            combatCount = combatCount + 1
            debugLog("  -> COMBAT: skipping regen")
        elseif not canRegen then
            combatCount = combatCount + 1
            local waitTime = CONFIG.OUT_OF_COMBAT_TIME - timeSinceCombat
            debugLog("  -> COOLDOWN: need " .. waitTime .. "s more out of combat")
        elseif stamina >= CONFIG.MAX_STAMINA then
            maxedCount = maxedCount + 1
            debugLog("  -> MAXED: already at max stamina")
        elseif isHappyHour then
            -- Happy hour zone (40-42h): slower regen rate
            -- We run every 2 min, but happy hour needs 4 min per stamina
            -- So we only regen every OTHER tick for happy hour players
            local happyHourTick = player:getStorageValue(STORAGE_HAPPY_HOUR_TICK) or 0
            if happyHourTick == 0 then
                player:setStorageValue(STORAGE_HAPPY_HOUR_TICK, 1)
                happyHourSkip = happyHourSkip + 1
                debugLog("  -> HAPPY HOUR: waiting (tick 1/2)")
            else
                player:setStorageValue(STORAGE_HAPPY_HOUR_TICK, 0)
                local newStamina = math.min(CONFIG.MAX_STAMINA, stamina + CONFIG.STAMINA_PER_TICK)
                player:setStamina(newStamina)
                regenCount = regenCount + 1
                debugLog("  -> HAPPY HOUR REGEN: " .. stamina .. " -> " .. newStamina .. " (+" .. CONFIG.STAMINA_PER_TICK .. ") (tick 2/2)")
            end
        else
            -- Normal zone (0-40h): normal regen rate
            local newStamina = math.min(CONFIG.MAX_STAMINA, stamina + CONFIG.STAMINA_PER_TICK)
            player:setStamina(newStamina)
            regenCount = regenCount + 1
            debugLog("  -> REGEN: " .. stamina .. " -> " .. newStamina .. " (+" .. CONFIG.STAMINA_PER_TICK .. ")")
        end
    end
    
    debugLog("Summary: " .. playerCount .. " players | " .. regenCount .. " regened | " .. combatCount .. " in combat | " .. maxedCount .. " maxed | " .. happyHourSkip .. " happy hour wait")
    debugLog("=========================")
    
    return true
end

onlineRegen:interval(CONFIG.REGEN_INTERVAL)
onlineRegen:register()
