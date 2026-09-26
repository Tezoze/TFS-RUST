local moveevent = MoveEvent()

function moveevent.onStepIn(creature, item, position, fromPosition)
	if creature:isPlayer() and not Game.isItemInPosition({x = 32259, y = 31891, z = 10}, 2129) then
		doRelocate({x = 32259, y = 31891, z = 10},{x = 32259, y = 31892, z = 10})
		Game.createItem(2129, 1, {x = 32259, y = 31891, z = 10})
	end
end

moveevent:aid(3010)
moveevent:register()