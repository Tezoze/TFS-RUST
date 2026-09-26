local moveevent = MoveEvent()

function moveevent.onStepIn(creature, item, position, fromPosition)
	if creature:isPlayer() and Game.isItemInPosition({x = 32479, y = 31920, z = 07},3696) and Game.isItemInPosition ({x = 32478, y = 31920, z = 07},3696) and Game.isItemInPosition ({x = 32478, y = 31902, z = 07}, 1791) then
		Game.transformItemInPosition({x = 32478, y = 31902, z = 07}, 1791, 1947)
		Game.sendMagicEffect({x = 32478, y = 31902, z = 07}, 3)
	end
end

moveevent:aid(3035)
moveevent:register()