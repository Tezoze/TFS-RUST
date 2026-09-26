local action = Action()

function action.onUse(player, item, fromPosition, target, toPosition)
	if item:getId() == 2772 and Game.isItemInPosition({x = 32568, y = 32078, z = 12},2185) and Game.isItemInPosition ({x = 32569, y = 32078, z = 12},2185) then
		item:transform(2773, 1)
		item:decay()
		Game.removeItemInPosition({x = 32568, y = 32078, z = 12}, 2185)
		Game.removeItemInPosition({x = 32569, y = 32078, z = 12}, 2185)
	elseif item:getId() == 2773 and Game.isItemInPosition({x = 32568, y = 32078, z = 12},2185) and Game.isItemInPosition ({x = 32569, y = 32078, z = 12}, 2185) then 
		item:transform(2772, 1)
		item:decay()
	elseif item:getId() == 2773 then
		item:transform(2772, 1)
		item:decay()
		Game.createItem(2185, 1, {x = 32568, y = 32078, z = 12})
		Game.createItem(2185, 1, {x = 32569, y = 32078, z = 12})
	end
	return true
end

action:aid(2011)
action:register()