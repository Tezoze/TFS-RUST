function onStepIn(creature, item, position, fromPosition)
	if not creature:isPlayer() then
		return true
	end

	local player = creature
	local arenaId = item.uid - 23200

	if arenaId >= player:getStorageValue(Storage.SvargrondArena.Arena) then
		return true
	end

	local cStorage = ARENA[arenaId].reward.trophyStorage
	if player:getStorageValue(cStorage) ~= 1 then
		local playerPos = player:getPosition()
		local trophyTilePos = Position(playerPos.x, playerPos.y, playerPos.z)

		-- Check multiple positions around the player, avoiding the trophy tile itself
		-- Prioritize north position first (where the counter is)
		local positionsToTry = {
			{x = playerPos.x, y = playerPos.y - 1, z = playerPos.z}, -- North (highest priority - counter location)
			{x = playerPos.x, y = playerPos.y + 1, z = playerPos.z}, -- South
			{x = playerPos.x - 1, y = playerPos.y, z = playerPos.z}, -- West
			{x = playerPos.x + 1, y = playerPos.y, z = playerPos.z}, -- East
			{x = playerPos.x - 1, y = playerPos.y - 1, z = playerPos.z}, -- Northwest
			{x = playerPos.x + 1, y = playerPos.y - 1, z = playerPos.z}, -- Northeast
			{x = playerPos.x - 1, y = playerPos.y + 1, z = playerPos.z}, -- Southwest
			{x = playerPos.x + 1, y = playerPos.y + 1, z = playerPos.z}, -- Southeast
		}

		-- Force placement on north position (highest priority)
		local northPos = {x = playerPos.x, y = playerPos.y - 1, z = playerPos.z}
		local validPosition = northPos

		local rewardItem = Game.createItem(ARENA[arenaId].reward.trophy, 1, validPosition)

		if rewardItem then
			player:setStorageValue(cStorage, 1)
			player:getPosition():sendMagicEffect(CONST_ME_MAGIC_RED)

			-- Try to set description after a short delay
			addEvent(function()
				if rewardItem then
					local description = string.format(ARENA[arenaId].reward.desc, player:getName())
					pcall(function()
						rewardItem:setDescription(description)
					end)
				end
			end, 100)
		else
			player:sendTextMessage(MESSAGE_EVENT_ADVANCE, 'Unable to create trophy. Please contact an administrator.')
		end
	end
	return true
end
