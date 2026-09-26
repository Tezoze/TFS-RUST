local moveevent = MoveEvent()

function moveevent.onStepIn(creature, item, position, fromPosition)
	if creature:isPlayer() and Game.isItemInPosition({x = 32266, y = 31892, z = 12}, 394) then 
		Game.transformItemInPosition({x = 32266, y = 31892, z = 12}, 394, 355)
	end
end

moveevent:aid(3009)
moveevent:register()