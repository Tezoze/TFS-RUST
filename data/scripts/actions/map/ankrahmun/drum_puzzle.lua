local action = Action()

function action.onUse(player, item, fromPosition, target, toPosition)
	if player:getStorageValue(259) == -1 then
		item:getPosition():sendMagicEffect(23)
		player:setStorageValue(259, 1)
	else
		item:getPosition():sendMagicEffect(23)
		player:setStorageValue(259, -1)
	end
	return true
end

action:aid(2070)
action:register()