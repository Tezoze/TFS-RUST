local moveevent = MoveEvent()

function moveevent.onAddItem(item, tileitem, position)
	if Game.isItemInPosition({x = 32476, y = 31900, z = 05},2471) and not Game.isItemInPosition ({x = 32478, y = 31904, z = 05}, 1948) then 
		Game.createItem(1948, 1, {x = 32478, y = 31904, z = 05})
		Game.transformItemInPosition({x = 32476, y = 31900, z = 05}, 431, 430)
	elseif Game.isItemInPosition({x = 32476, y = 31900, z = 05}, 2471) then
		Game.transformItemInPosition({x = 32476, y = 31900, z = 05}, 431, 430)
	end
end

moveevent:aid(3048)
moveevent:tileItem(true)
moveevent:register()

local moveevent = MoveEvent()

function moveevent.onRemoveItem(item, tileitem, position)
	if Game.isItemInPosition({x = 32478, y = 31904, z = 05}, 1948) then 
		Game.removeItemInPosition({x = 32478, y = 31904, z = 05}, 1948)
		Game.transformItemInPosition({x = 32476, y = 31900, z = 05}, 430, 431)
	else
		Game.transformItemInPosition({x = 32476, y = 31900, z = 05}, 430, 431)
	end
end

moveevent:aid(3048)
moveevent:tileItem(true)
moveevent:register()