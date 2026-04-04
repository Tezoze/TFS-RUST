function onUse(player, item, fromPosition, target, toPosition, isHotkey)
	if player:getStorageValue(Storage.ChildrenoftheRevolution.StrangeSymbols) == 2 then
		player:setStorageValue(Storage.ChildrenoftheRevolution.StrangeSymbols, 3)
		player:addItem(11106, 1)
		player:say("Somewhere between the flasks and dust the shelf also holds a gooey jar of extra greasy oil.", TALKTYPE_MONSTER_SAY)
	else
		player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "The storage is empty.")
	end
	return true
end