local action = Action()

function action.onUse(player, item, fromPosition, target, toPosition)
	return onUseMachete(player, item, fromPosition, target, toPosition)
end

action:id(3308, 3330)
action:register()