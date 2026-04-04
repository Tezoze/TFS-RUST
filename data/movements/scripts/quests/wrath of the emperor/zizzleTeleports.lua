local teleports = {
	[3186] = Position(33084, 31214, 8),
	[3187] = Position(33093, 31122, 12)
}

local scalePositions = {
	[3187] = {x = 33086, y = 31214, z = 8} -- Position where scale of corruption should be placed for teleport 3187
}

function onStepIn(creature, item, position, fromPosition)
	local player = creature:getPlayer()
	if not player then
		creature:teleportTo(fromPosition)
		return true
	end

	if player:getStorageValue(Storage.WrathoftheEmperor.Mission09) >= 2 then
		if item.uid == 3187 then
			-- For teleport 3187, check if scale of corruption is the top item at the specified position
			local scalePos = scalePositions[item.uid]
			if not scalePos then
				player:teleportTo(fromPosition)
				player:sendTextMessage(MESSAGE_EVENT_ADVANCE, 'Teleport system malfunction. Please try again later.')
				return true
			end

			-- Check if there's a scale on the ground at the expected position
			local scaleFound = false
			local posObj = Position(scalePos.x, scalePos.y, scalePos.z)
			local tile = Tile(posObj)

			if tile then
				-- Try to find the scale item at the position
				local tileItem = tile:getItemById(12629)
				if tileItem then
					scaleFound = true
					tileItem:remove()
				end
			end

			if scaleFound then
				player:teleportTo(teleports[item.uid])
				player:getPosition():sendMagicEffect(CONST_ME_TELEPORT)
			end
			-- If no scale found, do nothing - player can walk on the spot like a regular tile
		else
			-- For teleport 3186, always allow teleport (no item required)
			player:teleportTo(teleports[item.uid])
			player:getPosition():sendMagicEffect(CONST_ME_TELEPORT)
		end
	end
	return true
end
