function onUse(player, item, fromPosition, target, toPosition, isHotkey)
	if item.itemid ~= 8058 then
		return false
	end

	-- Check if player has completed all lever requirements but not yet the seal
	local allLeversComplete = true
	local leverRequirements = {
		[50015] = 1,
		[50016] = 2,
		[50017] = 3,
		[50018] = 4,
		[50019] = 5
	}

	for uid, required in pairs(leverRequirements) do
		local uses = player:getStorageValue("lever_" .. uid) or 0
		if uses < required then
			allLeversComplete = false
			break
		end
	end

	if not allLeversComplete then
		player:sendCancelMessage('The flame does not respond. You must complete the lever puzzle first.')
		return true
	end

	if player:getStorageValue(Storage.QueenOfBansheesQuest.ThirdSeal) >= 1 then
		player:sendCancelMessage('You have already completed this seal.')
		return true
	end

	-- Complete the Third Seal
	player:setStorageValue(Storage.QueenOfBansheesQuest.ThirdSeal, 1)

	-- Transform coal basins
	local coalBasins = {
		{x = 32214, y = 31850, z = 15},
		{x = 32215, y = 31850, z = 15},
		{x = 32216, y = 31850, z = 15}
	}

	local function doTransformCoalBasins(cbPos)
		local tile = Tile(cbPos)
		if tile then
			local thing = tile:getItemById(1485)
			if thing then
				thing:transform(1484)
			end
		end
	end

	for i = 1, #coalBasins do
		doTransformCoalBasins(coalBasins[i])
	end

	-- Remove the flame
	item:remove()

	player:sendTextMessage(MESSAGE_EVENT_ADVANCE, 'You have completed the Third Seal!')
	toPosition:sendMagicEffect(CONST_ME_FIREAREA)

	return true
end



