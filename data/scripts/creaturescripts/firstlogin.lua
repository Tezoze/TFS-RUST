local creatureevent = CreatureEvent("FirstLogin")

function creatureevent.onLogin(player)
	if player:getLastLoginSaved() == 0 then
		-- Explicit-slot adds fail (nil) when the slot is already occupied;
		-- never fall through to a Wherever add that would scatter starter
		-- gear over seeded/kept items (bench runes live in these hands).
		if not player:getSlotItem(CONST_SLOT_LEFT) then
			player:addItem(2382, 1, false, 1, CONST_SLOT_LEFT)
		end
		if not player:getSlotItem(CONST_SLOT_RIGHT) then
			player:addItem(2050, 1, false, 1, CONST_SLOT_RIGHT)
		end

		if not player:getSlotItem(CONST_SLOT_ARMOR) then
			if player:getSex() == PLAYERSEX_FEMALE then
				player:addItem(2485, 1, false, 1, CONST_SLOT_ARMOR)
			else
				player:addItem(2650, 1, false, 1, CONST_SLOT_ARMOR)
			end
		end

		if not player:getSlotItem(CONST_SLOT_BACKPACK) then
			local container = player:addItem(1987, 1, false, 1, CONST_SLOT_BACKPACK)
			if container then
				container:addItem(2674, 1)
			end
		end

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
