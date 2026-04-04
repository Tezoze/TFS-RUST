function onUse(player, item, fromPosition, target, toPosition, isHotkey)
	-- Inactive Lava Hole for Ice Islands Quest (Cure the Dogs mission)
	if target.itemid == 388 then
		if player:getStorageValue(Storage.TheIceIslands.Questline) == 23 then
			player:addItem(7247, 1) -- Fine Sulphur
			player:sendTextMessage(MESSAGE_INFO_DESCR, "You have extracted some fine sulphur from the inactive lava hole.")
			return true
		else
			return false -- Do nothing when not on the mission
		end
	end

	-- Giant Glimmercap Mushroom for Ice Islands Quest (Cure the Dogs mission)
	if target.itemid == 4184 then
		if player:getStorageValue(Storage.TheIceIslands.Questline) == 27 then
			player:addItem(7251, 1) -- Mushroom Spores
			player:sendTextMessage(MESSAGE_INFO_DESCR, "You have collected some mushroom spores from the giant glimmercap mushroom.")
			return true
		else
			return false -- Do nothing when not on the mission
		end
	end

	return false
end