local moveevent = MoveEvent()

function moveevent.onStepIn(creature, item, position, fromPosition)
	if not Game.isItemInPosition({x = 32266, y = 31861, z = 11}, 2772) then
		Game.transformItemInPosition({x = 32266, y = 31861, z = 11}, 2773, 2772)
		Game.transformItemInPosition({x = 32266, y = 31860, z = 11}, 411, 410)
		Game.createItem(2129, 1, {x = 32266, y = 31860, z = 11})
	end
end

moveevent:aid(3024)
moveevent:register()

local moveevent = MoveEvent()

function moveevent.onAddItem(item, tileitem, position)
	if not Game.isItemInPosition({x = 32266, y = 31861, z = 11}, 2772) then
		Game.transformItemInPosition({x = 32266, y = 31861, z = 11}, 2773, 2772)
		Game.transformItemInPosition({x = 32266, y = 31860, z = 11}, 411, 410)
		Game.createItem(2129, 1, {x = 32266, y = 31860, z = 11})
	end
end

moveevent:aid(3024)
moveevent:tileItem(true)
moveevent:register()