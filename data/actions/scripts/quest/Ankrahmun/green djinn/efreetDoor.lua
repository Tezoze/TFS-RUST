function onUse(player, item, fromPosition, target, toPosition, isHotkey)
	if item.itemid ~= 1255 then
		return true
	end

	if player:getStorageValue(Storage.DjinnWar.MaridFaction.Mission02) >= 1 then
		player:teleportTo(toPosition, true)
		item:transform(1256)
	else
		player:sendTextMessage(MESSAGE_INFO_DESCR, "The door seems to be sealed against unwanted intruders.")
	end
	return true
end