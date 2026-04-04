function onUse(player, item, fromPosition, target, toPosition, isHotkey)
	if player:getStorageValue(Storage.WrathoftheEmperor.Questline) >= 34 then
		player:teleportTo(toPosition, true)
		item:transform(item.itemid + 1)
		addEvent(function()
			local tile = Tile(fromPosition)
			if tile then
				local doorItem = tile:getItemById(item.itemid + 1)
				if doorItem and not tile:getTopCreature() then
					doorItem:transform(item.itemid)
				end
			end
		end, 1000)
	else
		player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "The door seems to be sealed against unwanted intruders.")
	end
	return true
end
