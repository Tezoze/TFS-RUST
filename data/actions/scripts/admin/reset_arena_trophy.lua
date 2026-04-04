function onUse(player, item, fromPosition, toPosition, isHotkey)
	local target = player:getTarget()
	if not target or not target:isPlayer() then
		player:sendTextMessage(MESSAGE_STATUS_CONSOLE_BLUE, "You need to target a player to reset their arena trophy storage.")
		return true
	end

	local arenaType = item:getAttribute("description") or "greenhorn"
	local storageKey

	if arenaType == "greenhorn" then
		storageKey = Storage.SvargrondArena.TrophyGreenhorn
	elseif arenaType == "scrapper" then
		storageKey = Storage.SvargrondArena.TrophyScrapper
	elseif arenaType == "warlord" then
		storageKey = Storage.SvargrondArena.TrophyWarlord
	else
		player:sendTextMessage(MESSAGE_STATUS_CONSOLE_BLUE, "Invalid arena type. Use 'greenhorn', 'scrapper', or 'warlord'.")
		return true
	end

	target:setStorageValue(storageKey, 0)
	player:sendTextMessage(MESSAGE_STATUS_CONSOLE_BLUE, "Reset " .. target:getName() .. "'s " .. arenaType .. " trophy storage.")
	target:sendTextMessage(MESSAGE_EVENT_ADVANCE, "Your " .. arenaType .. " trophy storage has been reset. You can now try claiming the trophy again.")
	return true
end


