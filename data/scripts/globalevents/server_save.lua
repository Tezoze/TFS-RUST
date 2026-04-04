-- Automatic Server Save GlobalEvent
-- Runs at 10:00 AM server time

local serverSave = GlobalEvent("ServerSave")

function serverSave.onTime(interval)
    local remainingTime = configManager.getNumber(configKeys.SERVER_SAVE_NOTIFY_DURATION) * 60000
    local minutesLeft = math.floor(remainingTime / 60000)
    local minuteText = (minutesLeft == 1) and "minute" or "minutes"
    local endMessage = (minutesLeft == 1) and "Please logout." or "Please come back in 10 minutes."

    if configManager.getBoolean(configKeys.SERVER_SAVE_NOTIFY_MESSAGE) then
        Game.broadcastMessage("Server is saving game in " .. minutesLeft .. " " .. minuteText .. ". " .. endMessage, MESSAGE_STATUS_WARNING)
    end

    addEvent(serverSaveWarning, 60000, remainingTime)
    return not configManager.getBoolean(configKeys.SERVER_SAVE_SHUTDOWN)
end

function serverSaveWarning(time)
    local remainingTime = tonumber(time) - 60000
    local minutesLeft = math.floor(remainingTime / 60000)
    local minuteText = (minutesLeft == 1) and "minute" or "minutes"
    local endMessage = (minutesLeft == 1) and "Please logout." or "Please come back in 10 minutes."

    if configManager.getBoolean(configKeys.SERVER_SAVE_NOTIFY_MESSAGE) then
        Game.broadcastMessage("Server is saving game in " .. minutesLeft .. " " .. minuteText .. ". " .. endMessage, MESSAGE_STATUS_WARNING)
    end

    if remainingTime > 60000 then
        addEvent(serverSaveWarning, 60000, remainingTime)
    else
        addEvent(executeServerSave, 60000)
    end
end

function executeServerSave()
    if configManager.getBoolean(configKeys.SERVER_SAVE_CLEAN_MAP) then
        cleanMap()
    end

    if configManager.getBoolean(configKeys.SERVER_SAVE_CLOSE) then
        Game.setGameState(GAME_STATE_CLOSED)
    end

    if configManager.getBoolean(configKeys.SERVER_SAVE_SHUTDOWN) then
        Game.setGameState(GAME_STATE_SHUTDOWN)
    end
end

serverSave:time("10:00:00")
serverSave:register()
