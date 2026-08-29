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

	-- Apply NPC promotion storage; native `premiumPromotion` gates perks (no login demotion).
	local isPromoted = player:getStorageValue(PlayerStorageKeys.promotion)
	if isPromoted == 1 then
		local vocation = player:getVocation()
		local promotion = vocation:getPromotion()
		local premiumGated = configManager.getBoolean(configKeys.PREMIUM_PROMOTION)
		if promotion and (player:isPremium() or not premiumGated) then
			player:setVocation(promotion)
		end
	end

	player:registerEvent("PlayerDeath")
	return true
end

creatureevent:register()
