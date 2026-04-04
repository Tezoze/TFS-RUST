local rewards = {
	[4022] = 12643, -- Replace with actual item ID for chest 1
	[4023] = 12642, -- Replace with actual item ID for chest 2
	[4024] = 12645  -- Replace with actual item ID for chest 3
}

function onUse(player, item, fromPosition, target, toPosition, isHotkey)
	if (player:getStorageValue(Storage.WrathoftheEmperor.mainReward) or 0) < 1 then
		player:setStorageValue(Storage.WrathoftheEmperor.mainReward, 1)
		player:addItem(rewards[item.uid], 1)
		player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "You have found " .. ItemType(rewards[item.uid]):getName() .. ".")
	else
		player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "The chest is empty.")
	end
	return true
end

