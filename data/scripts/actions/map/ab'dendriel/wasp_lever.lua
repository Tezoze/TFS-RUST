local action = Action()

function action.onUse(player, item, fromPosition, target, toPosition)
	if item:getId() == 2772 and Game.isItemInPosition({x = 32627, y = 31699, z = 10}, 1771) then
		item:transform(2773, 1)
		item:decay()
		doRelocate({x = 32627, y = 31699, z = 10},{x = 32626, y = 31699, z = 10})
		doRelocate({x = 32628, y = 31699, z = 10},{x = 32626, y = 31699, z = 10})
		doRelocate({x = 32629, y = 31699, z = 10},{x = 32626, y = 31699, z = 10})
		Game.transformItemInPosition({x = 32627, y = 31699, z = 10}, 1771, 622)
		Game.createItem(4788, 1, {x = 32627, y = 31699, z = 10})
		Game.transformItemInPosition({x = 32628, y = 31699, z = 10}, 1771, 622)
		Game.transformItemInPosition({x = 32629, y = 31699, z = 10}, 1771, 622)
		Game.createItem(4786, 1, {x = 32629, y = 31699, z = 10})
	elseif item:getId() == 2772 then
		item:transform(2773, 1)
		item:decay()
	elseif item:getId() == 2773 and Game.isItemInPosition({x = 32628, y = 31699, z = 10}, 622) then
		item:transform(2772, 1)
		item:decay()
		Game.transformItemInPosition({x = 32627, y = 31699, z = 10}, 622, 1771)
		Game.transformItemInPosition({x = 32628, y = 31699, z = 10}, 622, 1771)
		Game.transformItemInPosition({x = 32629, y = 31699, z = 10}, 622, 1771)
		Game.removeItemInPosition({x = 32627, y = 31699, z = 10}, 4788)
		Game.removeItemInPosition({x = 32629, y = 31699, z = 10}, 4786)
	end
	return true
end

action:aid(2001)
action:register()