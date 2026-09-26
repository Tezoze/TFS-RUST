local action = Action()

function action.onUse(player, item, fromPosition, target, toPosition)
	return onUseScythe(player, item, fromPosition, target, toPosition)
end

action:id(3453)
action:register()