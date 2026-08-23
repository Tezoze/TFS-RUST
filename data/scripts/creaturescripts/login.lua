local creatureevent = CreatureEvent("PlayerLogin")

function creatureevent.onLogin(player)
	local loginStr = "Welcome to " .. configManager.getString(configKeys.SERVER_NAME) .. "!"
	if player:getLastLoginSaved() <= 0 then
		loginStr = loginStr .. " Please choose your outfit."
		player:sendOutfitWindow()
	else
		if loginStr ~= "" then
			player:sendTextMessage(MESSAGE_STATUS_DEFAULT, loginStr)
		end

		loginStr = string.format("Your last visit was on %s.", os.date("%a %b %d %X %Y", player:getLastLoginSaved()))
	end
	player:sendTextMessage(MESSAGE_STATUS_DEFAULT, loginStr)

	-- Promotion
	local isPromoted = player:getStorageValue(PlayerStorageKeys.promotion)
	local vocation = player:getVocation()
	local promotion = vocation:getPromotion()
	if player:isPremium() then
		if isPromoted == 1 then
			player:setVocation(promotion)
		end
	elseif isPromoted == 1 and not player:isPremium() then
		player:setVocation(vocation:getDemotion())
	end

	player:registerEvent("PlayerDeath")
	return true
end

creatureevent:register()
