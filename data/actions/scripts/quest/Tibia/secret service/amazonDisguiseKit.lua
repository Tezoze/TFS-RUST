local condition = Condition(CONDITION_OUTFIT)
condition:setTicks(20 * 1000) -- should be approximately 20 seconds
condition:setOutfit({lookType = 137, lookHead = 113, lookBody = 120, lookLegs = 114, lookFeet = 132}) -- amazon looktype

function onUse(player, item, fromPosition, target, toPosition, isHotkey)
	player:addCondition(condition)
	player:say('You disguise yourself as a beautiful amazon!', TALKTYPE_MONSTER_SAY)
	player:setStorageValue(Storage.secretService.AmazonDisguiseKit, 1)

	-- Remove the disguise storage value after 20 seconds when the condition expires
	addEvent(function()
		local playerObj = Player(player:getId())
		if playerObj then
			playerObj:setStorageValue(Storage.secretService.AmazonDisguiseKit, -1)
		end
	end, 20000) -- 20 seconds = 20000 milliseconds

	item:remove()
	return true
end
