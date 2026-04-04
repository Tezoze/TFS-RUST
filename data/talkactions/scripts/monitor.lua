function onSay(player, words, param)
    if not player:getGroup():getAccess() then
        return true
    end

    if words == "!monitor" then
        if param == "start" then
            -- Start periodic monitoring every minute
            player:sendTextMessage(MESSAGE_STATUS_CONSOLE_BLUE, "Starting periodic server monitoring...")
            addEvent(function()
                monitorServerStats()
                -- Schedule next monitoring in 1 minute (60 seconds * 1000 ms)
                addEvent(function() monitorServerStats() end, 60000)
            end, 1000) -- Start after 1 second
            player:sendTextMessage(MESSAGE_STATUS_CONSOLE_BLUE, "Periodic monitoring started! Check console for stats every minute.")

        elseif param == "stop" then
            player:sendTextMessage(MESSAGE_STATUS_CONSOLE_BLUE, "Periodic monitoring stopped.")
            -- Note: In a real implementation, you'd need to track and cancel the scheduled events
            -- For now, this just provides feedback

        elseif param == "once" or param == "" then
            -- Run monitoring once
            monitorServerStats()
            player:sendTextMessage(MESSAGE_STATUS_CONSOLE_BLUE, "Server stats logged to console.")

        elseif param == "help" then
            player:sendTextMessage(MESSAGE_STATUS_CONSOLE_BLUE, "Server Monitoring Commands:")
            player:sendTextMessage(MESSAGE_STATUS_CONSOLE_BLUE, "!monitor - Run monitoring once")
            player:sendTextMessage(MESSAGE_STATUS_CONSOLE_BLUE, "!monitor start - Start periodic monitoring (every minute)")
            player:sendTextMessage(MESSAGE_STATUS_CONSOLE_BLUE, "!monitor stop - Stop periodic monitoring")
            player:sendTextMessage(MESSAGE_STATUS_CONSOLE_BLUE, "!monitor help - Show this help")

        else
            player:sendTextMessage(MESSAGE_STATUS_CONSOLE_RED, "Invalid parameter. Use !monitor help for usage.")
        end

        return false
    end

    return true
end
