local action = Action()

function action.onUse(player, item, fromPosition, target, toPosition)
	item:transform(3700)
	item:decay()
	Game.createItem(3588, 3, fromPosition)
	return true
end

action:id(3699)
action:register()
