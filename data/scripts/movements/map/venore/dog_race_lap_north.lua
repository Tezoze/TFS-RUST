local moveevent = MoveEvent()

function moveevent.onStepIn(creature, item, position, fromPosition)
	item:transform(430, 1)
	item:decay()
	Game.transformItemInPosition({x = 32915, y = 32078, z = 05}, 2114, 2113)
end

moveevent:aid(3119)
moveevent:register()

local moveevent = MoveEvent()

function moveevent.onStepOut(creature, item, position, fromPosition)
	item:transform(431, 1)
	item:decay()
	Game.transformItemInPosition({x = 32915, y = 32078, z = 05}, 2113, 2114)
end

moveevent:aid(3119)
moveevent:register()