function onUse(player, item, fromPosition, target, toPosition, isHotkey)
	if player:getStorageValue(Storage.SvargrondArena.Pit) == 1 then
		if item.itemid == 5132 then -- Closed door
			player:teleportTo(toPosition, true)
			item:transform(item.itemid + 1) -- Open the door
		else -- Door is already open, just teleport
			player:teleportTo(toPosition, true)
		end
	else
		player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "You need to pay Halvar first to enter the arena.")
	end
	return true
end
