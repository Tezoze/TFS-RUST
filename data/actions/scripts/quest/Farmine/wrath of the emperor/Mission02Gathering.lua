function onUse(player, item, fromPosition, target, toPosition, isHotkey)
	-- Use rake on mud to get lump of clay
	if item.itemid == 2549 and target.itemid == 12322 then -- corrupted mud
		if player:getStorageValue(Storage.WrathoftheEmperor.Mission02) == 1 then
			player:say("You carefully rake the mud and obtain a lump of clay.", TALKTYPE_MONSTER_SAY)
			player:addItem(12285, 1) -- lump of clay
		else
			player:say("This mud doesn't seem special enough to rake.", TALKTYPE_MONSTER_SAY)
		end
		return true
	end

	return false
end
