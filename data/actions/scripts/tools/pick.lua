function onUse(player, item, fromPosition, target, toPosition, isHotkey)

	-- Wrath of the Emperor Mission 02: Check for sacred wood gathering first
	if target.itemid == 12296 and player:getStorageValue(Storage.WrathoftheEmperor.Mission02) == 1 then -- big table with crack
		player:say("You carefully pick at the crack and obtain some unworked sacred wood.", TALKTYPE_MONSTER_SAY)
		player:addItem(12295, 1) -- unworked sacred wood
		return true
	end

	-- Special case: Beregar quest crack (fenrock cracks)
	-- Check if we're targeting rock soil (4409) and there's a crack (6299) on top of it
	if target.itemid == 4409 then
		local tile = Tile(toPosition)
		if tile then
			local topItem = tile:getTopTopItem()
			
			-- Check if there's a crack (6299) on this tile
			if topItem and topItem.itemid == 6299 and topItem.uid == 53118 then
				local storage = player:getStorageValue(Storage.hiddenCityOfBeregar.WayToBeregar) or 0
				
				-- Check if player asked Maris about Mistrock first
				if storage < 1 then
					player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "The crack is too small to be of any use. (You need to ask Maris about Mistrock first)")
					return true
				end

				-- Teleport player directly to Beregar
				toPosition:sendMagicEffect(CONST_ME_POFF)
				player:teleportTo(Position(32563, 31337, 10))
				player:getPosition():sendMagicEffect(CONST_ME_TELEPORT)
				player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "You broke through the crack and found a hidden passage!")
				
				-- Set storage to indicate access to Beregar is now available
				player:setStorageValue(Storage.hiddenCityOfBeregar.DefaultStart, 1)
				return true
			end
		end
	end
	
	-- Also handle direct targeting of the crack
	if target.itemid == 6299 and target.uid == 53118 then
		local storage = player:getStorageValue(Storage.hiddenCityOfBeregar.WayToBeregar) or 0
		
		-- Check if player asked Maris about Mistrock first
		if storage < 1 then
			player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "The crack is too small to be of any use. (You need to ask Maris about Mistrock first)")
			return true
		end

		-- Teleport player directly to Beregar
		toPosition:sendMagicEffect(CONST_ME_POFF)
		player:teleportTo(Position(32563, 31337, 10))
		player:getPosition():sendMagicEffect(CONST_ME_TELEPORT)
		player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "You broke through the crack and found a passage!")
		
		-- Set storage to indicate access to Beregar is now available
		player:setStorageValue(Storage.hiddenCityOfBeregar.DefaultStart, 1)
		return true
	end

	-- Pits of Inferno Quest: Lava cooling with pick on stone
	if target.uid == 1022 then
		local lavaPositions = { --Poi
			Position(32808, 32336, 11),
			Position(32809, 32336, 11),
			Position(32810, 32336, 11),
			Position(32808, 32334, 11),
			Position(32807, 32334, 11),
			Position(32807, 32335, 11),
			Position(32807, 32336, 11),
			Position(32807, 32337, 11),
			Position(32806, 32337, 11),
			Position(32805, 32337, 11),
			Position(32805, 32338, 11),
			Position(32805, 32339, 11),
			Position(32806, 32339, 11),
			Position(32806, 32338, 11),
			Position(32807, 32338, 11),
			Position(32808, 32338, 11),
			Position(32808, 32337, 11),
			Position(32809, 32337, 11),
			Position(32810, 32337, 11),
			Position(32811, 32337, 11),
			Position(32811, 32338, 11),
			Position(32806, 32338, 11),
			Position(32810, 32338, 11),
			Position(32810, 32339, 11),
			Position(32809, 32339, 11),
			Position(32809, 32338, 11),
			Position(32811, 32336, 11),
			Position(32811, 32335, 11),
			Position(32810, 32335, 11),
			Position(32809, 32335, 11),
			Position(32808, 32335, 11),
			Position(32809, 32334, 11),
			Position(32809, 32333, 11),
			Position(32810, 32333, 11),
			Position(32811, 32333, 11),
			Position(32806, 32338, 11),
			Position(32810, 32334, 11),
			Position(32811, 32334, 11),
			Position(32812, 32334, 11),
			Position(32813, 32334, 11),
			Position(32814, 32334, 11),
			Position(32812, 32333, 11),
			Position(32810, 32334, 11),
			Position(32812, 32335, 11),
			Position(32813, 32335, 11),
			Position(32814, 32335, 11),
			Position(32814, 32333, 11),
			Position(32813, 32333, 11)
		}

		for i = 1, #lavaPositions do
			Game.createItem(5815, 1, lavaPositions[i])
		end

		target:transform(2256)
		toPosition:sendMagicEffect(CONST_ME_POFF)
		return true
	end

	-- Transformable tiles: action IDs that transform to item ID 392 and decay after 1 minute
	local transformableTiles = {
		[101] = true,
		[103] = true,
		[104] = true,
		[105] = true,
		[107] = true,
		[109] = true,
		[111] = true,
		[112] = true,
		[113] = true,
		[115] = true,
		[117] = true,
		[119] = true,
		[120] = true,
		[121] = true,
		[122] = true,
		[123] = true,
		[124] = true,
		[125] = true,
		[126] = true,
		[127] = true,
		[128] = true,
		[129] = true,
		[130] = true,
		[131] = true,
		[132] = true,
		[133] = true,
		[134] = true,
		[135] = true,
		[136] = true,
		[137] = true,
		[138] = true,
		[139] = true,
		[140] = true,
		[141] = true,
		[142] = true,
		[143] = true,
		[144] = true,
		[145] = true,
		[146] = true,
		[147] = true,
		[148] = true,
		[149] = true,
		[150] = true,
		[151] = true,
		[152] = true,
		[163] = true
		-- Add more action IDs here as needed: [actionID] = true,
	}

	-- Check if the target has a transformable action ID
	if target.actionid and transformableTiles[target.actionid] then
		-- Get the tile and its current item ID and action ID for restoration
		local tile = Tile(toPosition)
		if tile then
			local originalItemId = target.itemid
			local originalActionId = target.actionid

			-- Transform the tile to item ID 392
			target:transform(392)

			-- Send visual effect
			toPosition:sendMagicEffect(CONST_ME_POFF)

			-- Schedule the decay back to original tile after 1 minute (60 seconds)
			addEvent(function()
				local tileToRestore = Tile(toPosition)
				if tileToRestore then
					local currentItem = tileToRestore:getGround()
					if currentItem and currentItem.itemid == 392 then
						currentItem:transform(originalItemId)
						currentItem:setActionId(originalActionId)
						toPosition:sendMagicEffect(CONST_ME_POFF)
					end
				end
			end, 60000) -- 60 seconds = 1 minute

			return true
		end
	end

	-- Ice Islands Quest: Ice Breaking
	local tile = Tile(toPosition)
	local hasIceCrack = false
	if tile then
		-- Check all items at this position for ice cracks
		for _, item in ipairs(tile:getItems()) do
			if item.itemid == 7185 or item.itemid == 3621 then
				hasIceCrack = true
				break
			end
		end
		-- Also check ground
		local ground = tile:getGround()
		if ground and (ground.itemid == 7185 or ground.itemid == 3621) then
			hasIceCrack = true
		end
	end

	if hasIceCrack then
		-- Check if we're at one of the 3 specific ice crack locations and get the passage index
		local targetPos = toPosition
		local validPositions = {
			{x = 32399, y = 31051, z = 7}, -- Passage 1
			{x = 32394, y = 31062, z = 7}, -- Passage 2
			{x = 32393, y = 31072, z = 7}  -- Passage 3
		}

		local passageIndex = nil
		for i, pos in ipairs(validPositions) do
			if targetPos.x == pos.x and targetPos.y == pos.y and targetPos.z == pos.z then
				passageIndex = i
				break
			end
		end

		if not passageIndex then
			return false -- Not at a quest ice crack location
		end

		-- Check if player has started the mission
		local mission02 = player:getStorageValue(Storage.TheIceIslands.Mission02)
		if mission02 < 1 then
			player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "You don't have a mission to break ice passages.")
			return true
		end

		-- Check if player has already completed breaking all ice passages
		if mission02 >= 4 then
			player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "You have already broken all the ice passages.")
			return true
		end

		-- Check if this specific passage has already been broken
		local passageStorageKey = "IcePassage" .. passageIndex
		if player:getStorageValue(Storage.TheIceIslands[passageStorageKey]) == 1 then
			player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "You have already broken this ice passage.")
			return true
		end

		-- Find and transform the specific 7185 ice crack item
		local iceCrackItem = nil
		local alreadyBroken = false

		-- Search through all items at this position for the 7185 ice crack
		for _, item in ipairs(tile:getItems()) do
			if item.itemid == 7185 then
				iceCrackItem = item
				break
			elseif item.itemid == 7186 then
				-- Found a broken ice crack (7186)
				alreadyBroken = true
				break
			end
		end

		-- Also check ground for 7185
		if not iceCrackItem and not alreadyBroken then
			local ground = tile:getGround()
			if ground then
				if ground.itemid == 7185 then
					iceCrackItem = ground
				elseif ground.itemid == 7186 then
					alreadyBroken = true
				end
			end
		end

		-- Check if this ice crack has already been broken
		if alreadyBroken then
			player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "This ice crack is already broken and will close soon.")
			return true
		end

		-- Check if we found the ice crack to break
		if not iceCrackItem then
			player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "There is no ice crack here to break.")
			return true
		end

		-- Mark this passage as broken
		player:setStorageValue(Storage.TheIceIslands[passageStorageKey], 1)

		-- Break the ice crack - transform 7185 to 7186
		iceCrackItem:transform(7186)

		-- Spawn several chakoyas around the crack
		local chakoyaTypes = {"Chakoya Toolshaper", "Chakoya Tribewarden", "Chakoya Windcaller"}
		for i = 1, 3 do
			local offsetX = math.random(-2, 2)
			local offsetY = math.random(-2, 2)
			local spawnPos = Position(toPosition.x + offsetX, toPosition.y + offsetY, toPosition.z)

			-- Make sure we don't spawn on the crack position itself
			if offsetX ~= 0 or offsetY ~= 0 then
				local chakoyaType = chakoyaTypes[math.random(#chakoyaTypes)]
				Game.createMonster(chakoyaType, spawnPos)
			end
		end

		-- Send effect and message
		toPosition:sendMagicEffect(CONST_ME_HITAREA)
		player:say("You broke the ice and several chakoyas appeared!", TALKTYPE_MONSTER_SAY)

		-- Check how many passages have been broken
		local brokenCount = 0
		for i = 1, 3 do
			local passageKey = "IcePassage" .. i
			if player:getStorageValue(Storage.TheIceIslands[passageKey]) == 1 then
				brokenCount = brokenCount + 1
			end
		end

		-- Update mission progress based on how many passages are broken
		player:setStorageValue(Storage.TheIceIslands.Mission02, brokenCount + 1)

		-- Update quest log based on progress
		if brokenCount == 1 then
			player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "You have broke 1 of 3 ice passages.")
		elseif brokenCount == 2 then
			player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "You have broke 2 of 3 ice passages.")
		elseif brokenCount == 3 then
			player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "You have broke 3 of 3 ice passages! Tell Hjaern your mission!")
			player:setStorageValue(Storage.TheIceIslands.Questline, 4)
			player:setStorageValue(Storage.TheIceIslands.Mission02, 4)
		end

		-- Schedule the crack to close after 1 minute (60 seconds)
		addEvent(function()
			local tile = Tile(toPosition)
			if tile then
				-- Find the broken ice crack (7186) to restore to 7185
				local crackItem = nil
				for _, item in ipairs(tile:getItems()) do
					if item.itemid == 7186 then
						crackItem = item
						break
					end
				end
				-- Also check ground
				if not crackItem then
					local ground = tile:getGround()
					if ground and ground.itemid == 7186 then
						crackItem = ground
					end
				end

				if crackItem then
					crackItem:transform(7185) -- Restore to small crack
					toPosition:sendMagicEffect(CONST_ME_POFF)
				end
			end
		end, 60000) -- 60 seconds = 1 minute

		return true
	end

	-- Default pick behavior for everything else
	return onUsePick(player, item, fromPosition, target, toPosition, isHotkey)
end
