function onUse(player, item, fromPosition, target, toPosition, isHotkey)
	-- Check if this is a key being used on a katana quest door
	if item:getId() == 2086 or item:getId() == 2087 or item:getId() == 2088 or item:getId() == 2089 then -- Common key IDs
		if target and target:getActionId() == 1003 then
			-- Unlock the door by changing actionID to 1005
			target:setActionId(1005)
			player:sendTextMessage(MESSAGE_INFO_DESCR, "You unlock the door with your master key.")
			return true
		end
	end

	-- Default behavior for locked doors
	player:sendTextMessage(MESSAGE_INFO_DESCR, "This door is locked.")
	return true
end
