local templePortal = Action()

function templePortal.onUse(player, item, fromPosition, target, toPosition, isHotkey)
	local temple = player:getTown():getTemplePosition()
	local playerPos = player:getPosition()
	player:teleportTo(temple)
	temple:sendMagicEffect(CONST_ME_TELEPORT)
	playerPos:sendMagicEffect(CONST_ME_POFF)
	item:remove()
	return true
end

templePortal:aid(30001)
templePortal:register()
