local talkaction = TalkAction("/refresh")

function talkaction.onSay(player, words, param)
	if not player:getGroup():getAccess() then
		return true
	end

	if player:getAccountType() < ACCOUNT_TYPE_GOD then
		return false
	end

	local tileCount = refreshMap()
	if tileCount > 0 then
		player:sendTextMessage(MESSAGE_STATUS_WARNING, "Refreshed " .. tileCount .. " tile" .. (tileCount > 1 and "s" or "") .. " from the map.")
	end
	return false
end

talkaction:register()