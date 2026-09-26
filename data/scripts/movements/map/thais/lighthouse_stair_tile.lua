local moveevent = MoveEvent()

function moveevent.onStepIn(creature, item, position, fromPosition)
	if creature:isPlayer() then 
		item:transform(430, 1)
		item:decay()
		Game.transformItemInPosition({x = 32225, y = 32282, z = 09}, 429, 438)
	end
end

moveevent:aid(3033)
moveevent:register()

local moveevent = MoveEvent()

function moveevent.onStepOut(creature, item, position, fromPosition)
	if creature:isPlayer() then 
		item:transform(431, 1)
		item:decay()
		Game.transformItemInPosition({x = 32225, y = 32282, z = 09}, 438, 429)
	end
end

moveevent:aid(3033)
moveevent:register()