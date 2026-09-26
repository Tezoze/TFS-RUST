local moveevent = MoveEvent()

function moveevent.onStepIn(creature, item, position, fromPosition)
	if creature:isPlayer() and Game.isItemInPosition({x = 32255, y = 31837, z = 09},2026) then 
		Game.sendMagicEffect({x = 32255, y = 31837, z = 09}, 3)
		Game.removeItemInPosition({x = 32255, y = 31837, z = 09}, 2026)
		doRelocate({x = 32255, y = 31837, z = 10},{x = 32255, y = 31837, z = 09})
	end
end

moveevent:aid(3025)
moveevent:register()