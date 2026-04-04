-- Blood Brothers Quest - Marziel's Diary
-- Mission 6: A Black History - Finding Marziel's diary

function onUse(player, item, fromPosition, target, toPosition, isHotkey)
	if player:getStorageValue(Storage.BloodBrothers.Mission06) == 1 then
		-- Complete Mission 6
		player:setStorageValue(Storage.BloodBrothers.Mission06, 2)
		player:addItem(1972, 1) -- Ancient book
		player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "You have found Marziel's diary! It reveals the dark history of the vampire brothers.")
		item:remove() -- Remove the diary from the ground
	else
		player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "This diary is written in an ancient script you cannot understand yet.")
	end
	
	return true
end
