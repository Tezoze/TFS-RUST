local function ServerSave()
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

local function ServerSaveWarning(time)
	local remainingTime = tonumber(time) - 60000
	local minutesLeft = math.floor(remainingTime / 60000)
	local minuteText = (minutesLeft == 1) and "minute" or "minutes"

	if configManager.getBoolean(configKeys.SERVER_SAVE_NOTIFY_MESSAGE) then
		Game.broadcastMessage("Server is saving game in " .. minutesLeft .. " " .. minuteText .. ". Please logout.", MESSAGE_STATUS_WARNING)
	end

	if remainingTime > 60000 then
		addEvent(ServerSaveWarning, 60000, remainingTime)
	else
		addEvent(ServerSave, 60000)
	end
end

function onTime(interval)
	local remainingTime = configManager.getNumber(configKeys.SERVER_SAVE_NOTIFY_DURATION) * 60000
	local minutesLeft = math.floor(remainingTime / 60000)
	local minuteText = (minutesLeft == 1) and "minute" or "minutes"

	if configManager.getBoolean(configKeys.SERVER_SAVE_NOTIFY_MESSAGE) then
		Game.broadcastMessage("Server is saving game in " .. minutesLeft .. " " .. minuteText .. ". Please logout.", MESSAGE_STATUS_WARNING)
	end

	addEvent(ServerSaveWarning, 60000, remainingTime)
	return not configManager.getBoolean(configKeys.SERVER_SAVE_SHUTDOWN)
end
