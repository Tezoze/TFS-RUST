local moveevent = MoveEvent()

function moveevent.onStepIn(creature, item, position, fromPosition)
	if creature:isPlayer() and Game.isItemInPosition({x = 32592, y = 31787, z = 04},3695) and creature:getPlayer():getStorageValue(45) < 1 then
		Game.removeItemInPosition({x = 32592, y = 31787, z = 04}, 3695)
		Game.sendMagicEffect({x = 32592, y = 31787, z = 04}, 15)
		doRelocate({x = 32592, y = 31787, z = 04},{x = 32593, y = 31787, z = 04})
		Game.createItem(3696, 1, {x = 32592, y = 31787, z = 04})
	end
end

moveevent:aid(3002)
moveevent:register()