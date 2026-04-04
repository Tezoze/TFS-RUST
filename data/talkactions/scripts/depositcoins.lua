local STORAGE_SHOP_POINTS = 10001

function onSay(player, words, param)
	if not player:getGroup():getAccess() and words ~= "!depositcoins" then
		return true
	end

	local usage = "Usage: /depositcoins <amount> or !depositcoins <amount>"

	-- Split parameters by space
	local params = param:splitTrimmed(" ")
	local amount = tonumber(params[1])

	if not amount or amount <= 0 then
		player:sendTextMessage(MESSAGE_STATUS_WARNING, usage)
		return false
	end

	-- Check if player has enough Store Coins (ID: 24774)
	local storeCoins = player:getItemCount(24774)
	if storeCoins < amount then
		player:sendTextMessage(MESSAGE_STATUS_WARNING, "You don't have enough Store Coins. You need " .. amount .. " but only have " .. storeCoins .. ".")
		return false
	end

	-- Remove the coins from inventory
	local removed = player:removeItem(24774, amount)
	if removed then
		-- Add points to storage
		local currentPoints = player:getStorageValue(STORAGE_SHOP_POINTS)
		if currentPoints == -1 then
			currentPoints = 0 -- First time depositing
		end
		player:setStorageValue(STORAGE_SHOP_POINTS, currentPoints + amount)

		player:sendTextMessage(MESSAGE_INFO_DESCR, "You have deposited " .. amount .. " Store Coins and received " .. amount .. " shop points.")
	else
		player:sendTextMessage(MESSAGE_STATUS_WARNING, "Failed to deposit Store Coins.")
	end

	return false
end
