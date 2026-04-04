local condition = Condition(CONDITION_OUTFIT)
condition:setOutfit({lookTypeEx = 12284})
condition:setTicks(-1)

function onUse(player, item, fromPosition, target, toPosition, isHotkey)
	local questline = player:getStorageValue(Storage.WrathoftheEmperor.Questline)
	local crateStatus = player:getStorageValue(Storage.WrathoftheEmperor.CrateStatus)

	-- Check if player has completed the quest or not started it
	if questline < 1 then
		player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "You don't know what this crate is for yet.")
		return true
	elseif questline > 3 then
		player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "You've already completed the mission that required this disguise.")
		return true
	end

	-- Check if player is on the right quest stage for using the disguise
	if questline ~= 2 then
		player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "Now is not the right time to use this disguise.")
		return true
	end

	-- Check if disguise is currently active (player is already disguised)
	if crateStatus == 2 then
		-- Allow player to exit disguise by using crate again
		local outfitCondition = player:getCondition(CONDITION_OUTFIT)
		if outfitCondition then
			player:removeCondition(outfitCondition)
		end
		player:setStorageValue(Storage.WrathoftheEmperor.CrateStatus, 1) -- 1 = disguise used but not active
		player:getPosition():sendMagicEffect(CONST_ME_POFF)
		player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "You crawl out of the crate, ending your disguise.")
		return true
	end

	-- Check if disguise was used before (crateStatus == 1) - allow re-use during quest
	if crateStatus == 1 then
		player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "You've tested the disguise before. You crawl back into the crate.")
	end

	-- Apply the disguise
	player:addCondition(condition)
	player:setStorageValue(Storage.WrathoftheEmperor.CrateStatus, 2) -- 2 = disguise active
	player:getPosition():sendMagicEffect(CONST_ME_POFF)
	player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "You crawl into the crate and pull the lid shut. To the outside world, you now appear as an ordinary crate.")
	return true
end
