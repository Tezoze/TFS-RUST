function onUse(player, item, fromPosition, target, toPosition, isHotkey)
	if item.itemid ~= 1257 then
		return true
	end

	if player:getStorageValue(Storage.DjinnWar.EfreetFaction.Mission02) >= 1 then
		player:teleportTo(toPosition, true)
		item:transform(1258)
	else
		player:sendTextMessage(MESSAGE_INFO_DESCR, "The door seems to be sealed against unwanted intruders.")
	end
	return true
end
