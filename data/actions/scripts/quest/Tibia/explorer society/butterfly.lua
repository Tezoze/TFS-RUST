function onUse(player, item, fromPosition, target, toPosition, isHotkey)
	if target.itemid == 4993 and player:getStorageValue(Storage.ExplorerSociety.TheButterflyHunt) == 8 then
		player:setStorageValue(Storage.ExplorerSociety.TheButterflyHunt, 9)
		toPosition:sendMagicEffect(CONST_ME_MAGIC_BLUE)
		item:transform(4866)
		target:remove()
	elseif target.itemid == 4994 and player:getStorageValue(Storage.ExplorerSociety.TheButterflyHunt) == 11 then
		player:setStorageValue(Storage.ExplorerSociety.TheButterflyHunt, 12)
		toPosition:sendMagicEffect(CONST_ME_MAGIC_BLUE)
		item:transform(4867)
		target:remove()
	elseif target.itemid == 4992 and player:getStorageValue(Storage.ExplorerSociety.TheButterflyHunt) == 14 then
		player:setStorageValue(Storage.ExplorerSociety.TheButterflyHunt, 15)
		toPosition:sendMagicEffect(CONST_ME_MAGIC_BLUE)
		item:transform(4868)
		target:remove()
	end
	return true
end
