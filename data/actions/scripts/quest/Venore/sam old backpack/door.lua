function onUse(player, item, fromPosition, target, toPosition, isHotkey)
	if item.itemid ~= 1225 then
		return true
	end

	if player:getStorageValue(Storage.SamsOldBackpack) >= 2 then
		player:teleportTo(toPosition, true)
		item:transform(1226)
	else
		player:sendTextMessage(MESSAGE_EVENT_DEFAULT, "The door seems to be sealed against unwanted intruders.")
	end
	return true
end
