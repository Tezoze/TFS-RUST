local action = Action()

function action.onUse(player, item, fromPosition, target, toPosition)
	if item:getId() == 3481 then
		if Tile(item:getPosition()):hasFlag(TILESTATE_PROTECTIONZONE) then
			item:getPosition():sendMagicEffect(CONST_ME_POFF)
			return true
		end

		item:transform(3482)
	else
		item:transform(3481)
		item:getPosition():sendMagicEffect(CONST_ME_POFF)
	end
	return true
end

action:id(3482)
action:id(3481)
action:register()
