function onUse(player, item, fromPosition, target, toPosition, isHotkey)
	-- Check if item being used is the scrying ball (item 11103)
	if item.itemid ~= 11103 then
		return false
	end

	-- Check if player has the required quest storage (mission 2)
	if player:getStorageValue(Storage.AnUneasyAlliance) ~= 3 then
		player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "You don't know what to do with this device.")
		return true
	end

	-- Check if the scrying ball is already shattered
	if item.itemid == 11104 then
		player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "The scrying ball is already destroyed.")
		return true
	end

	-- Transform the scrying ball to shattered state
	item:transform(11104)
	player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "You have successfully destroyed the scrying device!")

	-- Set storage to indicate mission completion
	player:setStorageValue(Storage.AnUneasyAlliance, 4)

	-- Schedule reversion after 10 seconds
	addEvent(function()
		local tile = Tile(toPosition)
		if tile then
			local revertedItem = tile:getItemById(11104)
			if revertedItem then
				revertedItem:transform(11103)
			end
		end
	end, 10000)

	return true
end
