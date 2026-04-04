local function revert(position, itemId, transformId)
	local item = Tile(position):getItemById(itemId)
	if item then
		item:transform(transformId)
	end
end

function onUse(player, item, fromPosition, target, toPosition, isHotkey)

	-- Check all items on the tile for shipyard action IDs
	local tile = Tile(toPosition)
	if tile then
		local items = tile:getItems()
		local hasShipyardTile = false
		for _, tileItem in ipairs(items) do
			if tileItem:getActionId() == 12550 or tileItem:getActionId() == 12551 then
				hasShipyardTile = true
				break
			end
		end

		if hasShipyardTile then
			-- Check if player has started TBI Mission 1
			if player:getStorageValue(Storage.secretService.TBIMission01) == 1 then
				-- Set mission in progress
				player:setStorageValue(Storage.secretService.TBIMission01, 2)
				-- Visual fire effect
				toPosition:sendMagicEffect(CONST_ME_FIREAREA)
				player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "You have successfully set fire to the Venorean shipyard! The docks will burn for several minutes.")
				return true
			else
				player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "You need to be on Chester Kahs' mission to burn the shipyard.")
				return false
			end
		end
	end

	if target.actionid == 54387 and target.itemid == 25531 then
		if player:getStorageValue(Storage.FerumbrasAscension.BasinCounter) >= 8 or player:getStorageValue(Storage.FerumbrasAscension.BoneFlute) < 1 then
			return false
		end
		if player:getStorageValue(Storage.FerumbrasAscension.BasinCounter) < 0 then
			player:setStorageValue(Storage.FerumbrasAscension.BasinCounter, 0)
		end
		if player:getStorageValue(Storage.FerumbrasAscension.BasinCounter) == 7 then
			player:say('You ascended the last basin.', TALKTYPE_MONSTER_SAY)
			item:remove()
			player:setStorageValue(Storage.FerumbrasAscension.MonsterDoor, 1)
		end
		target:transform(25532)
		player:setStorageValue(Storage.FerumbrasAscension.BasinCounter, player:getStorageValue(Storage.FerumbrasAscension.BasinCounter) + 1)
		toPosition:sendMagicEffect(CONST_ME_FIREAREA)
		addEvent(revert, 2 * 60 * 1000, toPosition, 25532, 25531)
		return true
	end
	
	if target.itemid == 5466 then		
		target:transform(5465)
		target:decay()
		return true
	end
	
	--Ballesta
	if target.itemid == 5697 then
		Game.createItem(5063, 1, toPosition)
		player:addExperience(100)
		if player:getStorageValue(45210) == 3 then
			player:setStorageValue(45210, 4)
			player:sendTextMessage(MESSAGE_CONSOLE_STATUS_ORANGE,'Good job, come back with baxter and tell him about your work.') 
		end
	end
	
	
	--Catapult	
	if target.itemid == 5609 then
		Game.createItem(5063, 1, toPosition)
		player:addExperience(100)
		if player:getStorageValue(45210) == 3 then
			player:sendTextMessage(MESSAGE_CONSOLE_STATUS_ORANGE,'Good job, come back with baxter and tell him about your work.') 
			player:setStorageValue(45210, 4)
		end
	end
	
	
	if target.actionid == 12550 or target.actionid == 12551 then
		-- DEBUG: Show target info
		player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "[DEBUG] Firebug used on actionid: " .. target.actionid .. ", itemid: " .. target.itemid)

		-- Check if player has started TBI Mission 1
		local missionStatus = player:getStorageValue(Storage.secretService.TBIMission01)
		player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "[DEBUG] TBIMission01 storage value: " .. missionStatus)

		if missionStatus == 1 then
			-- Check if fire is already burning (prevent spamming)
			local fireStorage = player:getStorageValue(12550) -- Use action ID as storage key
			local currentTime = os.time()
			player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "[DEBUG] Fire storage value: " .. fireStorage .. ", current time: " .. currentTime)

			if fireStorage > 0 and currentTime < fireStorage then
				player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "The docks are already burning. You need to wait until the fire dies down.")
				return false
			end

			-- Set mission in progress
			player:setStorageValue(Storage.secretService.TBIMission01, 2)
			-- Set fire burn time (7 minutes = 420 seconds)
			player:setStorageValue(12550, currentTime + 420)
			-- Visual fire effect
			toPosition:sendMagicEffect(CONST_ME_FIREAREA)
			player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "You have successfully set fire to the Venorean shipyard! The docks will burn for several minutes.")
			player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "[DEBUG] Fire set successfully! Mission status updated to 2.")
			return true
		else
			player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "You need to be on Chester Kahs' mission to burn the shipyard.")
			player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "[DEBUG] Mission not started - need TBIMission01 = 1")
			return false
		end
	end
	
	
	
	if target.uid == 2243 then
		Teleport = doCreateTeleport(1387, {x= 32857, y= 32234, z= 11},{x=32849, y=32233, z=9})
	end

	-- Blood Brothers Quest - Twines Fire Ritual
	if target.itemid >= 6220 and target.itemid <= 6225 then
		-- Check if player is on Mission 7 or higher and hasn't completed the twine ritual yet
		if player:getStorageValue(Storage.BloodBrothers.Mission07) < 1 or player:getStorageValue(Storage.BloodBrothers.BorekthKill) == 1 then
			player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "Nothing happens.")
			return true
		end

		-- Get current fire count
		local fireCount = player:getStorageValue(Storage.BloodBrothers.TwineFireCount)
		if fireCount < 0 then
			fireCount = 0
		end

		-- Increment fire count
		fireCount = fireCount + 1
		player:setStorageValue(Storage.BloodBrothers.TwineFireCount, fireCount)

		-- Display appropriate message based on fire count
		if fireCount == 1 then
			player:say("WHAT DO YOU THINK YOU ARE DOING TO MY PLANTS, INTRUDER?", TALKTYPE_MONSTER_SAY)
		elseif fireCount == 2 then
			player:say("YOU WILL DEARLY REGRET THIS, MORTAL.", TALKTYPE_MONSTER_SAY)
		end

		-- Remove fire bug
		item:remove(1)

		-- Send fire effect on the twines
		toPosition:sendMagicEffect(CONST_ME_HOLYAREA)

		-- Check if this is the 3rd use
		if fireCount >= 3 then
			-- Teleport to Boreth's room
			local borethRoomPosition = Position(32940, 31479, 1)
			player:teleportTo(borethRoomPosition)
			player:getPosition():sendMagicEffect(CONST_ME_TELEPORT)

			-- Spawn Boreth
			local borethPosition = Position(32940, 31476, 1)
			Game.createMonster('Boreth', borethPosition)

			player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "The ritual is complete! You have been teleported to a hidden chamber where Boreth awaits.")
			player:setStorageValue(Storage.BloodBrothers.TwineFireCount, 0) -- Reset counter
		end

		return true
	end

	-- General firebug functionality with random chance
	local chance = math.random(10)
	if chance > 4 then -- Success 6% chance
		if target.itemid == 7538 then -- Destroy spider webs/North - South
			toPosition:sendMagicEffect(CONST_ME_HITBYFIRE)
			target:transform(7544)
			target:decay()
			return true
		elseif target.itemid == 7539 then -- Destroy spider webs/East - West
			toPosition:sendMagicEffect(CONST_ME_HITBYFIRE)
			target:transform(7545)
			target:decay()
			return true
		elseif target.itemid == 1485 then -- Light up empty coal basins
			toPosition:sendMagicEffect(CONST_ME_HITBYFIRE)
			target:transform(1484)
			return true
		end
	elseif chance == 2 then -- It removes the firebug 1% chance
		item:remove(1)
		toPosition:sendMagicEffect(CONST_ME_POFF)
		return true
	elseif chance == 1 then -- It explodes on the user 1% chance
		doTargetCombat(0, player, COMBAT_FIREDAMAGE, -5, -5, CONST_ME_HITBYFIRE)
		player:say('OUCH!', TALKTYPE_MONSTER_SAY)
		item:remove(1)
		return true
	else
		toPosition:sendMagicEffect(CONST_ME_POFF) -- It fails, but don't get removed 3% chance
		return true
	end

	return true
end
