function onUse(player, item, fromPosition, target, toPosition, isHotkey)
	if player:getStorageValue(Storage.ChildrenoftheRevolution.Questline) == 9 then
		player:setStorageValue(Storage.ChildrenoftheRevolution.Questline, 10)
		player:addItem(10760, 1)
		player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "You have found a flask of poison.")
	else
		player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "The chest is empty.")
	end
	return true
end