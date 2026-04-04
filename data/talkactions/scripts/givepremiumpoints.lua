-- Give premium points to a player (admin command)
function onSay(player, words, param)
	if not player:getGroup():getAccess() then
		return true
	end

	local usage = "Usage: /givepremiumpoints <amount> <player name>"

	local params = param:splitTrimmed(" ")
	local amount = tonumber(params[1])

	-- Reconstruct player name from remaining parameters (handles spaces in names)
	local targetName
	if #params >= 2 then
		targetName = table.concat(params, " ", 2)
	end

	if not amount or amount <= 0 then
		player:sendTextMessage(MESSAGE_STATUS_WARNING, "Invalid amount. " .. usage)
		return false
	end

	if not targetName then
		player:sendTextMessage(MESSAGE_STATUS_WARNING, "Player name required. " .. usage)
		return false
	end

	local targetPlayer = Player(targetName)
	if not targetPlayer then
		player:sendTextMessage(MESSAGE_STATUS_WARNING, "Player '" .. targetName .. "' not found or offline.")
		return false
	end

	-- Get current premium points
	local accountId = targetPlayer:getAccountId()
	local resultId = db.storeQuery("SELECT `premium_points` FROM `accounts` WHERE `id` = " .. accountId .. " LIMIT 1")
	if not resultId then
		player:sendTextMessage(MESSAGE_STATUS_WARNING, "Failed to access account data.")
		return false
	end

	local currentPoints = result.getNumber(resultId, "premium_points") or 0
	result.free(resultId)

	-- Update premium points
	local newPoints = currentPoints + amount
	if not db.query("UPDATE `accounts` SET `premium_points` = " .. newPoints .. " WHERE `id` = " .. accountId) then
		player:sendTextMessage(MESSAGE_STATUS_WARNING, "Failed to add premium points.")
		return false
	end

	player:sendTextMessage(MESSAGE_INFO_DESCR, "Added " .. amount .. " premium points to " .. targetName .. ". They now have " .. newPoints .. " points.")
	targetPlayer:sendTextMessage(MESSAGE_INFO_DESCR, "You received " .. amount .. " premium points from an administrator. You now have " .. newPoints .. " points.")

	-- Log the action
	print("[ADMIN] " .. player:getName() .. " gave " .. amount .. " premium points to " .. targetName)

	return false
end
