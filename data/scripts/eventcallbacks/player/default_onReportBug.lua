local ec = EventCallback

ec.onReportBug = function(self, message)
	if self:getAccountType() == ACCOUNT_TYPE_NORMAL then
		return false
	end

	local name = self:getName()
	local playerPosition = self:getPosition()
	local ok = tfs.appendLog("bugs/" .. name .. " report.txt",
		"------------------------------\n" ..
		"Name: " .. name ..
		" [Player Position: " .. playerPosition.x .. ", " .. playerPosition.y .. ", " .. playerPosition.z .. "]\n" ..
		"Comment: " .. message .. "\n")

	if not ok then
		self:sendTextMessage(MESSAGE_EVENT_DEFAULT, "There was an error when processing your report, please contact a gamemaster.")
		return true
	end

	self:sendTextMessage(MESSAGE_EVENT_DEFAULT, "Your report has been sent to " .. configManager.getString(configKeys.SERVER_NAME) .. ".")
	return true
end

ec:register()
