function onUse(player, item, fromPosition, target, toPosition, isHotkey)
	local questline = player:getStorageValue(Storage.ChildrenoftheRevolution.Questline)

	-- Using oil on levers to grease them
	if item.itemid == 11106 and target.actionid == 8013 and questline == 13 then
		player:setStorageValue(Storage.ChildrenoftheRevolution.Questline, 14)
		player:setStorageValue(Storage.ChildrenoftheRevolution.Mission04, 4) --Questlog, Children of the Revolution "Mission 4: Zze Way of Zztonezz"
		player:say("Due to being extra greasy, the leavers can now be moved.", TALKTYPE_MONSTER_SAY)
		item:remove()

	-- Pulling levers after greasing to find combination
	elseif item.actionid == 8013 and questline == 14 then
		player:setStorageValue(Storage.ChildrenoftheRevolution.Questline, 17)
		player:setStorageValue(Storage.ChildrenoftheRevolution.Mission04, 5) --Questlog, Children of the Revolution "Mission 4: Zze Way of Zztonezz"
		player:say("You found the right combination. You should report to Zalamon.", TALKTYPE_MONSTER_SAY)
		item:transform(item.itemid == 10044 and 10045 or 10044)
	end
	return true
end
