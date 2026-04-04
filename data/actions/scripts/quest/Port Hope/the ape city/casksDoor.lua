function onUse(player, item, fromPosition, target, toPosition, isHotkey)
	if player:getStorageValue(Storage.TheApeCity.Questline) >= 13 then
		if item.itemid == 3551 then
			player:teleportTo(toPosition, true)
			item:transform(item.itemid + 1)
		end
	else
		player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "The door seems to be sealed against unwanted intruders.")
	end
	return true
end
