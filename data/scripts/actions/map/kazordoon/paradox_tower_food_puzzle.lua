local action = Action()

function action.onUse(player, item, fromPosition, target, toPosition)
	if item:getId() == 2772 and Game.isItemInPosition({x = 32476, y = 31900, z = 04},3593) and Game.isItemInPosition ({x = 32477, y = 31900, z = 04},3587) and Game.isItemInPosition ({x = 32478, y = 31900, z = 04},3590) and Game.isItemInPosition ({x = 32479, y = 31900, z = 04},3585) and Game.isItemInPosition ({x = 32480, y = 31900, z = 04},3592) and Game.isItemInPosition ({x = 32481, y = 31900, z = 04},3589) then
		Game.createItem(1948, 1, {x = 32476, y = 31904, z = 04})
		Game.removeItemInPosition({x = 32476, y = 31900, z = 04}, 3593)
		Game.removeItemInPosition({x = 32477, y = 31900, z = 04}, 3587)
		Game.removeItemInPosition({x = 32478, y = 31900, z = 04}, 3590)
		Game.removeItemInPosition({x = 32479, y = 31900, z = 04}, 3585)
		Game.removeItemInPosition({x = 32480, y = 31900, z = 04}, 3592)
		Game.removeItemInPosition({x = 32481, y = 31900, z = 04}, 3589)
		item:transform(2773, 1)
		item:decay()
	elseif item:getId() == 2772 then
		Game.sendMagicEffect({x = 32479, y = 31905, z = 04}, 3)
	end
	return true
end

action:aid(2049)
action:register()