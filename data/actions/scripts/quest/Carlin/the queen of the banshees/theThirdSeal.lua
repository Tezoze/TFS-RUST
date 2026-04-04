local function doTransformCoalBasins(cbPos)
	local tile = Tile(cbPos)
	if tile then
		local thing = tile:getItemById(1485)
		if thing then
			thing:transform(1484)
		end
	end
end

local config = {
	-- Lever requirements: UID -> required uses
	levers = {
		[50015] = 1,  -- Lever 50015 needs to be used 1 time
		[50016] = 2,  -- Lever 50016 needs to be used 2 times
		[50017] = 3,  -- Lever 50017 needs to be used 3 times
		[50018] = 4,  -- Lever 50018 needs to be used 4 times
		[50019] = 5   -- Lever 50019 needs to be used 5 times
	},
	coalBasins = {
		{x = 32214, y = 31850, z = 15},
		{x = 32215, y = 31850, z = 15},
		{x = 32216, y = 31850, z = 15}
	},
	effects = {
		[50015] = { -- Effects for lever 50015
			{x= 32217, y= 31845, z= 14},
			{x= 32218, y= 31845, z= 14},
			{x= 32219, y= 31845, z= 14}
		},
		[50016] = { -- Effects for lever 50016
			{x= 32217, y= 31844, z= 14},
			{x= 32218, y= 31844, z= 14},
			{x= 32219, y= 31843, z= 14}
		},
		[50017] = { -- Effects for lever 50017
			{x= 32217, y= 31842, z= 14},
			{x= 32219, y= 31843, z= 14},
			{x= 32219, y= 31845, z= 14}
		},
		[50018] = { -- Effects for lever 50018
			{x= 32217, y= 31845, z= 14},
			{x= 32218, y= 31846, z= 14},
			{x= 32218, y= 31844, z= 14}
		},
		[50019] = { -- Effects for lever 50019
			{x= 32219, y= 31841, z= 14},
			{x= 32219, y= 31842, z= 14},
			{x= 32219, y= 31846, z= 14}
		},
	},
}


function onUse(player, item, fromPosition, target, toPosition, isHotkey)
	local playerName = player:getName()
	local itemId = item.itemid
	local itemUid = item.uid
	local thirdSealComplete = player:getStorageValue(Storage.QueenOfBansheesQuest.ThirdSeal) or 0

	-- print(string.format("[ThirdSeal Debug] Player: %s, ItemID: %d, ItemUID: %d, ThirdSealComplete: %d",
	--	playerName, itemId, itemUid, thirdSealComplete))

	-- Check if lever UID is valid
	local requiredUses = config.levers[itemUid]
	if not requiredUses then
		player:sendCancelMessage('This lever is not part of the Third Seal puzzle.')
		return true
	end

	-- Check if quest is already completed
	if thirdSealComplete >= 1 then
		player:sendCancelMessage('You have already completed this seal.')
		return true
	end

	if item.itemid == 1946 then
		-- Check if this lever is globally completed
		local leverGloballyComplete = Game.getStorageValue("lever_complete_" .. itemUid) or 0
		if leverGloballyComplete >= 1 then
			player:sendCancelMessage('This lever has already been used the required number of times by the team.')
			return true
		else
			-- Allow resetting stuck levers back to unused state
			item:transform(1945)
			player:sendCancelMessage('Lever reset to unused state.')
			-- print(string.format("[ThirdSeal Debug] Lever %d reset from 1946 to 1945", itemUid))
			return true
		end
	end

	-- Get current global usage count for this lever (resets on server restart)
	local leverStorageKey = "lever_uses_" .. itemUid
	local currentUses = Game.getStorageValue(leverStorageKey) or 0

	-- Only activate if we haven't reached the required uses yet
	if currentUses >= requiredUses then
		player:sendCancelMessage(string.format('Lever %d has already been used the required %d times.', itemUid, requiredUses))
		return true
	end

	-- Increment global usage count
	currentUses = currentUses + 1
	Game.setStorageValue(leverStorageKey, currentUses)

	-- Transform lever to show animation on each use (toggle between states)
	-- When complete, keep in final position (1946)
	if currentUses >= requiredUses then
		item:transform(1946) -- Stay in final "used" position when complete
	elseif item.itemid == 1945 then
		item:transform(1946) -- Move to "used" position
	else
		item:transform(1945) -- Move back to "unused" position
	end

	-- Show effects and magic (disabled)
	-- toPosition:sendMagicEffect(CONST_ME_MAGIC_BLUE)

	-- Show energy effects around lever (disabled)
	-- if config.effects[itemUid] then
	--	for i = 1, #config.effects[itemUid] do
	--		Position(config.effects[itemUid][i]):sendMagicEffect(CONST_ME_ENERGYHIT)
	--	end
	-- end

	-- print(string.format("[ThirdSeal Debug] Lever %d activated globally (%d/%d)", itemUid, currentUses, requiredUses))

	-- Show progress message (disabled)
	if currentUses >= requiredUses then
		-- player:sendTextMessage(MESSAGE_EVENT_ADVANCE, string.format('You have completed the required uses for lever %d!', itemUid))
		-- Set global completion flag for this lever
		Game.setStorageValue("lever_complete_" .. itemUid, 1)
		-- print(string.format("[ThirdSeal Debug] Lever %d requirement met! Setting global flag.", itemUid))
	else
		-- player:sendTextMessage(MESSAGE_EVENT_ADVANCE, string.format('Lever %d progress: %d/%d', itemUid, currentUses, requiredUses))
	end

	-- Check if all levers are now complete
	local allComplete = true
	for uid, required in pairs(config.levers) do
		local isComplete = Game.getStorageValue("lever_complete_" .. uid) or 0
		if isComplete < 1 then
			allComplete = false
			break
		end
	end

	if allComplete then
		Game.setStorageValue(Storage.QueenOfBansheesQuest.ThirdSealActive, 1)
		-- Game.broadcastMessage("The mystic flame is now active!", MESSAGE_STATUS_WARNING)
		-- player:sendTextMessage(MESSAGE_EVENT_ADVANCE, 'The mystic flame is now active! Walk on it to complete the Third Seal.')
		-- print("[ThirdSeal Debug] All levers complete - mystic flame is now active")
	end

	return true
end
