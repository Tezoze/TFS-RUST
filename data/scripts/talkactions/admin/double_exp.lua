--[[
    Double XP Event Command
    Usage: /doublexp on|off|status|24h
    
    - /doublexp on      - Enable double XP indefinitely
    - /doublexp off     - Disable double XP
    - /doublexp status  - Check current status
    - /doublexp 24h     - Enable for 24 hours
    - /doublexp 12h     - Enable for 12 hours
    - /doublexp 1h      - Enable for 1 hour
]]

local STORAGE_DOUBLE_EXP = 39901        -- Global storage for enabled state
local STORAGE_DOUBLE_EXP_END = 39902    -- Global storage for end timestamp

local doubleExp = TalkAction("/doublexp", "!doublexp")

function doubleExp.onSay(player, words, param)
    if player:getGroup():getAccess() == false then
        return true
    end
    
    param = param:lower():trim()
    
    if param == "on" then
        Game.setStorageValue(STORAGE_DOUBLE_EXP, 1)
        Game.setStorageValue(STORAGE_DOUBLE_EXP_END, 0) -- No end time (permanent until turned off)
        
        -- Broadcast to all players
        Game.broadcastMessage("DOUBLE XP EVENT has been activated! All experience gains are now doubled!", MESSAGE_EVENT_ADVANCE)
        player:sendTextMessage(MESSAGE_STATUS_CONSOLE_BLUE, "Double XP enabled (no time limit).")
        
    elseif param == "off" then
        Game.setStorageValue(STORAGE_DOUBLE_EXP, 0)
        Game.setStorageValue(STORAGE_DOUBLE_EXP_END, 0)
        
        Game.broadcastMessage("The Double XP Event has ended. Experience rates have returned to normal.", MESSAGE_EVENT_ADVANCE)
        player:sendTextMessage(MESSAGE_STATUS_CONSOLE_BLUE, "Double XP disabled.")
        
    elseif param == "status" then
        local enabled = Game.getStorageValue(STORAGE_DOUBLE_EXP) == 1
        local endTime = Game.getStorageValue(STORAGE_DOUBLE_EXP_END)
        
        if not enabled then
            player:sendTextMessage(MESSAGE_STATUS_CONSOLE_BLUE, "Double XP is currently DISABLED.")
        elseif endTime > 0 then
            local remaining = endTime - os.time()
            if remaining > 0 then
                local hours = math.floor(remaining / 3600)
                local mins = math.floor((remaining % 3600) / 60)
                player:sendTextMessage(MESSAGE_STATUS_CONSOLE_BLUE, string.format("Double XP is ENABLED. Time remaining: %dh %dm", hours, mins))
            else
                player:sendTextMessage(MESSAGE_STATUS_CONSOLE_BLUE, "Double XP timer expired but still active. Use '/doublexp off' to disable.")
            end
        else
            player:sendTextMessage(MESSAGE_STATUS_CONSOLE_BLUE, "Double XP is ENABLED (no time limit).")
        end
        
    elseif param:match("^%d+h$") then
        local hours = tonumber(param:match("^(%d+)h$"))
        if not hours or hours < 1 or hours > 168 then -- Max 1 week
            player:sendTextMessage(MESSAGE_STATUS_CONSOLE_BLUE, "Invalid duration. Use 1-168 hours (e.g., /doublexp 24h)")
            return false
        end
        
        local endTime = os.time() + (hours * 3600)
        Game.setStorageValue(STORAGE_DOUBLE_EXP, 1)
        Game.setStorageValue(STORAGE_DOUBLE_EXP_END, endTime)
        
        Game.broadcastMessage(string.format("DOUBLE XP EVENT has been activated for %d hours! All experience gains are now doubled!", hours), MESSAGE_EVENT_ADVANCE)
        player:sendTextMessage(MESSAGE_STATUS_CONSOLE_BLUE, string.format("Double XP enabled for %d hours.", hours))
        
    else
        player:sendTextMessage(MESSAGE_STATUS_CONSOLE_BLUE, "Usage: /doublexp on|off|status|24h|12h|1h")
    end
    
    return false
end

doubleExp:separator(" ")
doubleExp:access(true)
doubleExp:accountType(ACCOUNT_TYPE_GOD)
doubleExp:register()

-- Global event to check and disable expired double XP
local checkDoubleExp = GlobalEvent("CheckDoubleExpEvent")

function checkDoubleExp.onThink(interval)
    local enabled = Game.getStorageValue(STORAGE_DOUBLE_EXP)
    local endTime = Game.getStorageValue(STORAGE_DOUBLE_EXP_END)
    
    if enabled == 1 and endTime > 0 and os.time() >= endTime then
        Game.setStorageValue(STORAGE_DOUBLE_EXP, 0)
        Game.setStorageValue(STORAGE_DOUBLE_EXP_END, 0)
        Game.broadcastMessage("The Double XP Event has ended. Experience rates have returned to normal.", MESSAGE_EVENT_ADVANCE)
        print("[Double XP] Event ended automatically after timer expired.")
    end
    
    return true
end

checkDoubleExp:interval(60000) -- Check every minute
checkDoubleExp:register()
