local ge = GlobalEvent("PlayerRecord")
function ge.onRecord(current, old)
	Game.broadcastMessage("New record: " .. current .. " players are logged in.", MESSAGE_STATUS_DEFAULT)
	return true
end
ge:type("record")
ge:register()
