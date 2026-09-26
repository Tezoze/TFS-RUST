local moveevent = MoveEvent()

function moveevent.onStepIn(creature, item, position, fromPosition)
	if creature:isPlayer() and Game.isItemInPosition({x = 32104, y = 32082, z = 07}, 4597) then 
		Game.transformItemInPosition({x = 32104, y = 32082, z = 07}, 4597, 4601)
	end
end

moveevent:aid(3013)
moveevent:register()