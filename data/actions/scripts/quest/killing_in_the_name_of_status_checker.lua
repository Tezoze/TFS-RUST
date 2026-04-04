-- Load the killing in the name of quest data
dofile('data/lib/quests/killing_in_the_name_of.lua')

function onUse(player, item, fromPosition, target, toPosition, isHotkey)
	-- Only work on violet gems with action ID 25000 (purchased from Grizzly Adams)
	if item:getActionId() ~= 25000 then
		return false -- Let the item behave normally
	end

	-- Rename the item to "task gem" for better identification
	if item:getName() ~= "task gem" then
		item:setAttribute(ITEM_ATTRIBUTE_NAME, "task gem")
	end

	-- Check if player joined the Paw & Fur society
	local joinStorage = player:getStorageValue(Storage.KillingInTheNameOf.Join)
	if joinStorage ~= 0 then
		player:sendTextMessage(MESSAGE_INFO_DESCR, "You haven't joined the Paw and Fur - Hunting Elite society yet. Speak to Grizzly Adams in Port Hope to join.")
		return true
	end

	local rank = player:getPawAndFurRank()
	local points = player:getPawAndFurPoints()
	local rankName = (rank == 6 and "Elite Hunter" or rank == 5 and "Trophy Hunter" or rank == 4 and "Big Game Hunter" or rank == 3 and "Ranger" or rank == 2 and "Huntsman" or rank == 1 and "Member" or "Not ranked")

	player:sendTextMessage(MESSAGE_INFO_DESCR, "Current Rank: " .. rankName .. " (" .. points .. " points)")

	-- Show active tasks
	local activeTasks = player:getStartedTasks()
	if #activeTasks > 0 then
		player:sendTextMessage(MESSAGE_INFO_DESCR, "\nActive Tasks:")
		for i, taskId in ipairs(activeTasks) do
			local currentKills = player:getStorageValue(Storage.KillingInTheNameOf.KillsStorageBase + taskId)
			local requiredKills = tasks[taskId].killsRequired
			local creatureName = tasks[taskId].raceName
			player:sendTextMessage(MESSAGE_INFO_DESCR, "  " .. creatureName .. ": " .. currentKills .. "/" .. requiredKills .. " kills")
		end
	else
		player:sendTextMessage(MESSAGE_INFO_DESCR, "No active task")
	end

	-- Special tasks and elite task sections removed as requested

	return true
end
