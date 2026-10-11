local channel = Channel(2, "Rule Violations")

function channel.canJoin(player)
	return player:getAccountType() >= ACCOUNT_TYPE_GAMEMASTER
end

function channel.onSpeak()
	return false
end

channel:register()
