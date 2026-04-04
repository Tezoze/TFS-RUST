local premiumScroll = Action()

function premiumScroll.onUse(player, item, fromPosition, target, toPosition, isHotkey)
	print("[DEBUG] Premium scroll used by " .. player:getName())
	item:remove(1)

	local success = player:addPremiumDays(30)
	print("[DEBUG] addPremiumDays result: " .. tostring(success))
	print("[DEBUG] Premium time after: " .. player:getPremiumTime())

	player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "You have activated your 30 day premium time, relog to make it effective.")
	player:getPosition():sendMagicEffect(CONST_ME_TELEPORT)
	return true
end

premiumScroll:id(16101)
premiumScroll:register()
