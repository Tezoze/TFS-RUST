function onSay(player, words, param)
	if not player:getGroup():getAccess() then
		return true
	end

	if param == "" then
		player:sendTextMessage(MESSAGE_STATUS_CONSOLE_BLUE, "Usage: /resetarena <greenhorn|scrapper|warlord>")
		return false
	end

	local storageKey
	if param == "greenhorn" then
		storageKey = Storage.SvargrondArena.TrophyGreenhorn
	elseif param == "scrapper" then
		storageKey = Storage.SvargrondArena.TrophyScrapper
	elseif param == "warlord" then
		storageKey = Storage.SvargrondArena.TrophyWarlord
	else
		player:sendTextMessage(MESSAGE_STATUS_CONSOLE_BLUE, "Invalid arena type. Use 'greenhorn', 'scrapper', or 'warlord'.")
		return false
	end

	player:setStorageValue(storageKey, 0)
	player:sendTextMessage(MESSAGE_STATUS_CONSOLE_BLUE, "Reset your " .. param .. " trophy storage to 0. You can now try claiming the trophy again.")
	return false
end


