function onUse(player, item, fromPosition, target, toPosition, isHotkey)
	-- Wrath of the Emperor Mission02
	if item.itemid == 2549 and target.itemid == 12322 then -- corrupted mud
		if player:getStorageValue(Storage.WrathoftheEmperor.Mission02) == 1 then
			player:addItem(12285, 1)
			player:say("You dig out a handful of ordinary clay.", TALKTYPE_MONSTER_SAY)
		else
			player:say("This mud doesn't seem special enough to rake.", TALKTYPE_MONSTER_SAY)
		end
		return true
	end

	-- The Shattered Isles Parrot ring
	if item.itemid == 2549 and target.itemid == 6094 then -- parrot statue
		if player:getStorageValue(Storage.TheShatteredIsles.TheGovernorDaughter) == 1 then
			toPosition:sendMagicEffect(CONST_ME_POFF)
			Game.createItem(6093, 1, Position(32422, 32770, 1))
			player:say("You have found a ring.", TALKTYPE_MONSTER_SAY)
			player:setStorageValue(Storage.TheShatteredIsles.TheGovernorDaughter, 2)
		else
			player:say("This statue doesn't seem special enough to rake.", TALKTYPE_MONSTER_SAY)
		end
		return true
	end

	return false
end