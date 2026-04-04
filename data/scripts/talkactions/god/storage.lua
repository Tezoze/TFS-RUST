-- Storage Talkaction (Revscript)
-- Usage: /storage PlayerName StorageKey Value
-- Example: /storage Zorro 12130 27

local storage = TalkAction("/storage")

function storage.onSay(player, words, param)
    -- Check if player is a god
    if player:getAccountType() < ACCOUNT_TYPE_GOD then
        player:sendCancelMessage("You cannot use this command.")
        return false
    end
    
    if param == "" then
        player:sendTextMessage(MESSAGE_STATUS_CONSOLE_BLUE, "Usage: /storage PlayerName StorageKey Value")
        return false
    end
    
    local split = param:split(" ")
    if #split < 3 then
        player:sendTextMessage(MESSAGE_STATUS_CONSOLE_BLUE, "Usage: /storage PlayerName StorageKey Value")
        return false
    end
    
    local targetName = split[1]
    local storageKey = tonumber(split[2])
    local storageValue = tonumber(split[3])
    
    if not storageKey or not storageValue then
        player:sendTextMessage(MESSAGE_STATUS_CONSOLE_BLUE, "Storage key and value must be numbers.")
        return false
    end
    
    local target = Player(targetName)
    if not target then
        player:sendTextMessage(MESSAGE_STATUS_CONSOLE_BLUE, "Player '" .. targetName .. "' not found or not online.")
        return false
    end
    
    local oldValue = target:getStorageValue(storageKey)
    target:setStorageValue(storageKey, storageValue)
    
    player:sendTextMessage(MESSAGE_STATUS_CONSOLE_BLUE, 
        string.format("Set storage %d = %d for %s (was: %d)", 
            storageKey, storageValue, target:getName(), oldValue))
    
    return false
end

storage:separator(" ")
storage:register()

