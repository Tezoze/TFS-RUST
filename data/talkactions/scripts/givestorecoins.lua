-- Note: givestorecoins gives physical coins, not points
-- Use /depositcoins or !depositcoins to convert coins to shop points

function onSay(player, words, param)
	if not player:getGroup():getAccess() then
		return true
	end

	local usage = "Usage: /givestorecoins <amount> [player name]"

	-- Split parameters by space
	local params = param:splitTrimmed(" ")
	local amount = tonumber(params[1])
	local targetPlayer = player

	if not amount or amount <= 0 then
		player:sendTextMessage(MESSAGE_STATUS_WARNING, usage)
		return false
	end

	-- Check if a player name was specified
	if #params >= 2 then
		local playerName = table.concat(params, " ", 2)
		targetPlayer = Player(playerName)
		if not targetPlayer then
			player:sendTextMessage(MESSAGE_STATUS_WARNING, "Player '" .. playerName .. "' not found.")
			return false
		end
	end

	-- Give Store Coins to the target player
	local added = targetPlayer:addItem(24774, amount)
	if added then
		if targetPlayer == player then
			player:sendTextMessage(MESSAGE_INFO_DESCR, "You have received " .. amount .. " Store Coins.")
		else
			player:sendTextMessage(MESSAGE_INFO_DESCR, "You have given " .. amount .. " Store Coins to " .. targetPlayer:getName() .. ".")
			targetPlayer:sendTextMessage(MESSAGE_INFO_DESCR, "You have received " .. amount .. " Store Coins from " .. player:getName() .. ".")
		end
	else
		player:sendTextMessage(MESSAGE_STATUS_WARNING, "Failed to add Store Coins. Player's inventory might be full.")
	end

	return false
end
