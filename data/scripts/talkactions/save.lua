local save = TalkAction("/save")

function save.onSay(player, words, param)
	if player:getGroup():getId() >= 4 then
		saveServer()
		player:sendTextMessage(MESSAGE_STATUS_CONSOLE_BLUE, "Server saved.")
	else
		player:sendCancelMessage("You cannot use this command.")
	end
	return false
end

save:register()
