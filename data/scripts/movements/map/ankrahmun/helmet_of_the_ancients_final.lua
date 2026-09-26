local moveevent = MoveEvent()

function moveevent.onAddItem(item, tileitem, position)
	if Game.isItemInPosition({x = 33198, y = 32876, z = 11},3222) and Game.isItemInPosition ({x = 33198, y = 32876, z = 11},3223) and Game.isItemInPosition ({x = 33198, y = 32876, z = 11},3224) and Game.isItemInPosition ({x = 33198, y = 32876, z = 11},3225) and Game.isItemInPosition ({x = 33198, y = 32876, z = 11},3226) and Game.isItemInPosition ({x = 33198, y = 32876, z = 11},3227) and Game.isItemInPosition ({x = 33198, y = 32876, z = 11},3228) then 
		Game.removeItemInPosition({x = 33198, y = 32876, z = 11}, 3222)
		Game.removeItemInPosition({x = 33198, y = 32876, z = 11}, 3223)
		Game.removeItemInPosition({x = 33198, y = 32876, z = 11}, 3224)
		Game.removeItemInPosition({x = 33198, y = 32876, z = 11}, 3225)
		Game.removeItemInPosition({x = 33198, y = 32876, z = 11}, 3226)
		Game.removeItemInPosition({x = 33198, y = 32876, z = 11}, 3227)
		Game.removeItemInPosition({x = 33198, y = 32876, z = 11}, 3228)
		Game.createItem(3229, 1, {x = 33198, y = 32876, z = 11})
		Game.sendMagicEffect({x = 33198, y = 32876, z = 11}, 7)
	end
end

moveevent:aid(3018)
moveevent:tileItem(true)
moveevent:register()