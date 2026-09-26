local action = Action()

function action.onUse(player, item, fromPosition, target, toPosition)
	if not target:isItem() then
		return false
	end
	
	if target:getId() == 2199 and toPosition.x == 32754 and toPosition.y == 32559 and toPosition.z == 9 and player:getStorageValue(315) == 1 then
		item:transform(4843, 1)
		item:decay()
		player:setStorageValue(316, 1)
		target:getPosition():sendMagicEffect(4)
		return true
	end
	return false
end

action:id(4842) -- sheet of tracing paper (empty)
action:register()
