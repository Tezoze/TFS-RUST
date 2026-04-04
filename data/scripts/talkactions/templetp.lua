local templetp = TalkAction("/templetp")

function templetp.onSay(player, words, param)
	if not player:getGroup():getAccess() then
		return true
	end

	local position = player:getPosition()
	position:getNextPosition(player:getDirection())

	local tile = Tile(position)
	if not tile then
		player:sendCancelMessage("No tile found.")
		return false
	end

	local thing = tile:getTopVisibleThing(player)
	if not thing or not thing:isItem() then
		player:sendCancelMessage("No item found.")
		return false
	end

	if thing == tile:getGround() then
		player:sendCancelMessage("Cannot use ground tile.")
		return false
	end

	-- Set actionid 30001 for temple teleport
	thing:setActionId(30001)
	position:sendMagicEffect(CONST_ME_MAGIC_BLUE)
	player:sendTextMessage(MESSAGE_STATUS_CONSOLE_BLUE, "Item transformed into temple teleport.")
	return false
end

templetp:separator(" ")
templetp:register()
