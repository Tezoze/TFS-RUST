local action = Action()

function action.onUse(player, item, fromPosition, target, toPosition)
	if item:getId() == 2772 and Game.isItemInPosition({x = 33148, y = 32867, z = 09}, 2129) and Game.isItemInPosition ({x = 33149, y = 32867, z = 09}, 2129) and Game.isItemInPosition ({x = 33148, y = 32868, z = 09}, 2129) and Game.isItemInPosition ({x = 33149, y = 32868, z = 09}, 2129) then 
		item:transform(2773, 1)
		item:decay()
		Game.removeItemInPosition({x = 33148, y = 32867, z = 09}, 2129)
		Game.removeItemInPosition({x = 33149, y = 32867, z = 09}, 2129)
		Game.removeItemInPosition({x = 33148, y = 32868, z = 09}, 2129)
		Game.removeItemInPosition({x = 33149, y = 32868, z = 09}, 2129)
	elseif item:getId() == 2772 then
		item:transform(2773, 1)
		item:decay()
	elseif item:getId() == 2773 and Game.isItemInPosition({x = 33148, y = 32867, z = 09}, 2129) then 
		item:transform(2772, 1)
		item:decay()
	elseif item:getId() == 2773 then 
		item:transform(2772, 1)
		item:decay()
		doRelocate({x = 33148, y = 32867, z = 09}, {x = 33148, y = 32869, z = 09})
		doRelocate({x = 33149, y = 32867, z = 09}, {x = 33149, y = 32869, z = 09})
		doRelocate({x = 33148, y = 32868, z = 09}, {x = 33148, y = 32869, z = 09})
		doRelocate({x = 33149, y = 32868, z = 09},{x = 33149, y = 32869, z = 09})
		Game.createItem(2129, 1, {x = 33148, y = 32867, z = 09})
		Game.createItem(2129, 1, {x = 33149, y = 32867, z = 09})
		Game.createItem(2129, 1, {x = 33148, y = 32868, z = 09})
		Game.createItem(2129, 1, {x = 33149, y = 32868, z = 09})
	end
	return true
end

action:aid(2003)
action:register()