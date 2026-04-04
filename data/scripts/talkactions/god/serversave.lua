-- Manual Server Save Talkaction
-- Usage: /serversave
-- Triggers the same 5-minute countdown as the automatic server save

local manualServerSave = TalkAction("/serversave")

function manualServerSave.onSay(player, words, param)
    if player:getAccountType() < ACCOUNT_TYPE_GOD then
        return false
    end

    local remainingTime = configManager.getNumber(configKeys.SERVER_SAVE_NOTIFY_DURATION) * 60000
    local minutesLeft = math.floor(remainingTime / 60000)
    local minuteText = (minutesLeft == 1) and "minute" or "minutes"

    player:sendTextMessage(MESSAGE_STATUS_CONSOLE_BLUE, "Manual server save initiated. Countdown started.")

    local endMessage = (minutesLeft == 1) and "Please logout." or "Please come back in 10 minutes."

    if configManager.getBoolean(configKeys.SERVER_SAVE_NOTIFY_MESSAGE) then
        Game.broadcastMessage("Server is saving game in " .. minutesLeft .. " " .. minuteText .. ". " .. endMessage, MESSAGE_STATUS_WARNING)
    end

    addEvent(serverSaveWarning, 60000, remainingTime)
    return false
end

manualServerSave:separator(" ")
manualServerSave:register()
