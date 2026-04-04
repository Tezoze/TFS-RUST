function onUse(player, item, fromPosition, target, toPosition, isHotkey)
	-- Special handling for rum casks in Ape City quest
	if target.itemid == 5539 and target.actionid == 100 then
		-- Check if player has accepted the casks mission
		if player:getStorageValue(Storage.TheApeCity.Questline) < 13 then
			player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "You don't have a reason to destroy this cask.")
			return true
		end

		-- Get current casks destroyed count
		local casksDestroyed = math.max(0, player:getStorageValue(Storage.TheApeCity.Casks))

		-- Destroy the cask
		target:remove(1)
		toPosition:sendMagicEffect(CONST_ME_POFF)

		-- Increment casks counter
		player:setStorageValue(Storage.TheApeCity.Casks, casksDestroyed + 1)

		-- Send message
		if casksDestroyed < 2 then
			player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "You destroy the cask. " .. (2 - casksDestroyed) .. " more to go.")
		else
			player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "You destroy the last cask. You should report back to Hairycles.")
		end

		return true
	end

	-- Sewer pipe UIDs for In Service of Yalahar quest
	local sewerPipes = {
		[3071] = {storage = Storage.InServiceofYalahar.SewerPipe01, message = "You loosen the pipe with the crowbar and clear it from garbage."}, -- Copper pipe (ID: 9351)
		[3072] = {storage = Storage.InServiceofYalahar.SewerPipe02, message = "You loosen the pipe with the crowbar and clear it from garbage."}, -- Mechanism (ID: 9531)
		[3073] = {storage = Storage.InServiceofYalahar.SewerPipe03, message = "You loosen the pipe with the crowbar and clear it from garbage."}, -- Iron pillar (ID: 9084)
		[3074] = {storage = Storage.InServiceofYalahar.SewerPipe04, message = "You loosen the pipe with the crowbar and clear it from garbage."}  -- Copper wheel
	}

	-- Check if target is a sewer pipe from In Service of Yalahar quest
	local pipe = sewerPipes[target.uid]
	if pipe then
		-- Check if player is on the correct quest stage
		local questline = player:getStorageValue(Storage.InServiceofYalahar.Questline) or 0
		if questline < 5 then
			player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "You don't know what to do with this pipe.")
			return true
		end

		-- Check if pipe is already cleaned
		if (player:getStorageValue(pipe.storage) or 0) == 1 then
			player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "This pipe has already been cleaned.")
			return true
		end

		-- Clean the pipe
		player:setStorageValue(pipe.storage, 1)
		player:sendTextMessage(MESSAGE_EVENT_ADVANCE, pipe.message)
		toPosition:sendMagicEffect(CONST_ME_BLOCKHIT)

		-- Check if all pipes are cleaned
		local pipe01 = player:getStorageValue(Storage.InServiceofYalahar.SewerPipe01) == 1 and 1 or 0
		local pipe02 = player:getStorageValue(Storage.InServiceofYalahar.SewerPipe02) == 1 and 1 or 0
		local pipe03 = player:getStorageValue(Storage.InServiceofYalahar.SewerPipe03) == 1 and 1 or 0
		local pipe04 = player:getStorageValue(Storage.InServiceofYalahar.SewerPipe04) == 1 and 1 or 0

		if pipe01 == 1 and pipe02 == 1 and pipe03 == 1 and pipe04 == 1 then
			player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "You have cleaned all 4 sewer pipes! Return to Palimuth to report your mission.")
		else
			local cleaned = pipe01 + pipe02 + pipe03 + pipe04
			player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "Pipes cleaned: " .. cleaned .. "/4")
		end

		return true
	end

	-- Special handling for Chester Kahs Mission 6 - Destroy beer cask in Svargrond
	if target.itemid == 4859 and target.actionid == 12566 then
		-- Check if player has started Mission 6
		if player:getStorageValue(Storage.secretService.TBIMission06) < 1 then
			player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "You don't have a reason to destroy this cask.")
			return true
		end

		-- Check if player has the amazon disguise kit
		if player:getStorageValue(Storage.secretService.AmazonDisguiseKit) ~= 1 then
			player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "You need to disguise yourself as an amazon first.")
			return true
		end

		-- Check if cask is already destroyed (status 2 = destroyed)
		if player:getStorageValue(12566) == 2 then
			player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "This cask has already been destroyed.")
			return true
		end

		-- Transform the cask into wooden trash for 1 minute
		target:transform(2250) -- Transform to wooden trash
		toPosition:sendMagicEffect(CONST_ME_BLOCKHIT)
		player:setStorageValue(Storage.secretService.TBIMission06, 2) -- Mark as destroyed
		player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "You successfully destroy the beer cask disguised as an amazon!")

		-- Spawn barbarian skullhunters
		local skullhunter1Pos = Position(32205, 31157, 8)
		local skullhunter2Pos = Position(32204, 31157, 8)

		Game.createMonster('Barbarian Skullhunter', skullhunter1Pos)
		Game.createMonster('Barbarian Skullhunter', skullhunter2Pos)

		-- Set up transformation back to cask after 1 minute (60 seconds)
		addEvent(function()
			local tile = Tile(toPosition)
			if tile then
				local trashItem = tile:getItemById(2250)
				if trashItem then
					trashItem:transform(4859) -- Transform back to cask
				end
			end
		end, 60000) -- 60 seconds = 60000 milliseconds

		return true
	end

	return onUseCrowbar(player, item, fromPosition, target, toPosition, isHotkey)
end
