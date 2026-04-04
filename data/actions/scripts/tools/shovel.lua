function onUse(player, item, fromPosition, target, toPosition, isHotkey)
	-- Wrath of the Emperor Mission 02: Check for sacred earth digging first
	if target.itemid == 351 and player:getStorageValue(Storage.WrathoftheEmperor.Mission02) == 1 then
		player:say("You dig up some earth from the sacred ground.", TALKTYPE_MONSTER_SAY)
		target:transform(12297)
		player:addItem(12297, 1) -- sacred earth
		target:transform(351) -- reset the earth after a delay
		addEvent(function() target:transform(351) end, 60000) -- 1 minute delay
		return true
	end

	return onUseShovel(player, item, fromPosition, target, toPosition, isHotkey)
end
