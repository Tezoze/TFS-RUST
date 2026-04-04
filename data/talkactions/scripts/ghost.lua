function onSay(player, words, param)
	if not player:getGroup():getAccess() then
		return true
	end

	if player:getGroup():getId() <= 4 then
		return false
	end

	local position = player:getPosition()
	local isGhost = not player:isInGhostMode()

	player:setGhostMode(isGhost)
	player:setPassThroughMode(isGhost)
	if isGhost then
		player:sendTextMessage(MESSAGE_INFO_DESCR, "You are now in ghost mode (invisible and can walk through walls).")
		position:sendMagicEffect(CONST_ME_YALAHARIGHOST)
	else
		player:sendTextMessage(MESSAGE_INFO_DESCR, "You are visible again and can no longer walk through walls.")
		position.x = position.x + 1
		position:sendMagicEffect(CONST_ME_SMOKE)
	end
	return false
end
