function onUse(player, item, fromPosition, target, toPosition, isHotkey)
	-- Sun Adorer Cactus for Ice Islands Quest (Cure the Dogs mission)
	if target.itemid == 2733 then
		if player:getStorageValue(Storage.TheIceIslands.Questline) == 21 then
			player:addItem(7245, 1) -- Part of Sun Adorer Cactus
			player:sendTextMessage(MESSAGE_INFO_DESCR, "You have harvested a part of the sun adorer cactus.")
			return true
		else
			return false -- Do nothing when not on the mission
		end
	end

	-- Frostbite Herb for Ice Islands Quest (Cure the Dogs mission)
	if target.itemid == 7261 then
		if player:getStorageValue(Storage.TheIceIslands.Questline) == 24 then
			player:addItem(7248, 1) -- Frostbite Herb
			player:sendTextMessage(MESSAGE_INFO_DESCR, "You have harvested a frostbite herb.")
			return true
		else
			return false -- Do nothing when not on the mission
		end
	end

	-- Purple Kiss Bush for Ice Islands Quest (Cure the Dogs mission)
	if target.itemid == 4017 then
		if player:getStorageValue(Storage.TheIceIslands.Questline) == 25 then
			player:addItem(7249, 1) -- Purple Kiss Blossom
			player:sendTextMessage(MESSAGE_INFO_DESCR, "You have harvested a blossom from the purple kiss bush.")
			return true
		else
			return false -- Do nothing when not on the mission
		end
	end

	return false
end
