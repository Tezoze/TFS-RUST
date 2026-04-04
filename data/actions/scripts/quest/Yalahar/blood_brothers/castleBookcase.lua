-- Blood Brothers Quest - Castle Bookcase
-- Mission 6: A Black History - Getting Arthei's diary part 1

function onUse(player, item, fromPosition, target, toPosition, isHotkey)
	if player:getStorageValue(Storage.BloodBrothers.Mission06) == 1 then
		-- Give first diary part
		player:setStorageValue(Storage.BloodBrothers.Mission06, 2) -- Advance to return phase
		player:addExperience(1200, true)
		player:addItem(2236, 1) -- Arthei's Descent into Vampirism I
		player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "You have found Arthei's Descent into Vampirism I! Take it back to Julius.")
	elseif player:getStorageValue(Storage.BloodBrothers.Mission06) >= 2 then
		player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "The bookcase is empty now.")
	else
		player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "This bookcase contains ancient tomes, but you're not sure what you're looking for yet.")
	end

	return false
end
