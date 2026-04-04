function onUse(player, item, fromPosition, target, toPosition, isHotkey)
	-- Check if player has discovered the deeper Banuta shortcut
	if player:getStorageValue(Storage.DeeperBanutaShortcut) == 1 then
		-- Allow passage/teleport
		return false
	else
		player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "You need to use the egg of The Many on the giant lizard head first.")
		return true
	end
end
