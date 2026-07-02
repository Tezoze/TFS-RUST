local creatureevent = CreatureEvent("FirstLogin")

function creatureevent.onLogin(player)
	if player:getLastLoginSaved() == 0 then
		player:addItem(2382, 1, false, 1, CONST_SLOT_LEFT)
		player:addItem(2050, 1, false, 1, CONST_SLOT_RIGHT)
		
		if player:getSex() == PLAYERSEX_FEMALE then
			player:addItem(2485, 1, false, 1, CONST_SLOT_ARMOR)
		else
			player:addItem(2650, 1, false, 1, CONST_SLOT_ARMOR)
		end
		
		local container = player:addItem(1987, 1, false, 1, CONST_SLOT_BACKPACK)
		container:addItem(2674, 1)
		
		-- default outfit
		if player:getSex() == PLAYERSEX_MALE then
			player:setOutfit({lookType=128, lookHead=78, lookBody=106, lookLegs=58, lookFeet=95})
		else
			player:setOutfit({lookType=136, lookHead=78, lookBody=106, lookLegs=58, lookFeet=95})
		end
		player:setDirection(DIRECTION_SOUTH)
	end
	return true
end

creatureevent:register()