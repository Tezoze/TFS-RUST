local channel = Channel(7, "Help")
channel:public(true)

-- TODO(chat CH-5): Uncomment when Condition API is available
-- local muted = Condition(CONDITION_CHANNELMUTEDTICKS, CONDITIONID_DEFAULT)
-- muted:setParameter(CONDITION_PARAM_SUBID, 7)
-- muted:setParameter(CONDITION_PARAM_TICKS, 3600000)

function onSpeak(player, type, message)
	local playerAccountType = player:getAccountType()
	if player:getLevel() == 1 and playerAccountType == ACCOUNT_TYPE_NORMAL then
		player:sendCancelMessage("You may not speak into channels as long as you are on level 1.")
		return false
	end

	-- TODO(chat CH-5): Uncomment when Condition API is available
	-- if player:getCondition(CONDITION_CHANNELMUTEDTICKS, CONDITIONID_DEFAULT, 7) then
	--     player:sendCancelMessage("You are muted from the Help channel for using it inappropriately.")
	--     return false
	-- end

	if playerAccountType >= ACCOUNT_TYPE_TUTOR then
		if string.sub(message, 1, 6) == "!mute " then
			-- TODO(chat CH-5): Uncomment when Condition API is available
			-- local targetName = string.sub(message, 7)
			-- local target = Player(targetName)
			-- if target then
			--     if playerAccountType > target:getAccountType() then
			--         if not target:getCondition(CONDITION_CHANNELMUTEDTICKS, CONDITIONID_DEFAULT, 7) then
			--             target:addCondition(muted)
			--             sendChannelMessage(7, TALKTYPE_CHANNEL_R1, target:getName() .. " has been muted by " .. player:getName() .. " for using Help Channel inappropriately.")
			--         else
			--             player:sendCancelMessage("That player is already muted.")
			--         end
			--     else
			--         player:sendCancelMessage("You are not authorized to mute that player.")
			--     end
			-- else
			--     player:sendCancelMessage(RETURNVALUE_PLAYERWITHTHISNAMEISNOTONLINE)
			-- end
			return false
		elseif string.sub(message, 1, 8) == "!unmute " then
			-- TODO(chat CH-5): Uncomment when Condition API is available
			-- local targetName = string.sub(message, 9)
			-- local target = Player(targetName)
			-- if target then
			--     if playerAccountType > target:getAccountType() then
			--         if target:getCondition(CONDITION_CHANNELMUTEDTICKS, CONDITIONID_DEFAULT, 7) then
			--             target:removeCondition(CONDITION_CHANNELMUTEDTICKS, CONDITIONID_DEFAULT, 7)
			--             sendChannelMessage(7, TALKTYPE_CHANNEL_R1, target:getName() .. " has been unmuted by " .. player:getName() .. ".")
			--         else
			--             player:sendCancelMessage("That player is not muted.")
			--         end
			--     else
			--         player:sendCancelMessage("You are not authorized to unmute that player.")
			--     end
			-- else
			--     player:sendCancelMessage(RETURNVALUE_PLAYERWITHTHISNAMEISNOTONLINE)
			-- end
			return false
		end
	end

	if type == TALKTYPE_CHANNEL_Y then
		if playerAccountType >= ACCOUNT_TYPE_TUTOR or player:hasFlag(PlayerFlag_TalkOrangeHelpChannel) then
			type = TALKTYPE_CHANNEL_O
		end
	elseif type == TALKTYPE_CHANNEL_O then
		if playerAccountType < ACCOUNT_TYPE_TUTOR and not player:hasFlag(PlayerFlag_TalkOrangeHelpChannel) then
			type = TALKTYPE_CHANNEL_Y
		end
	elseif type == TALKTYPE_CHANNEL_R1 then
		if playerAccountType < ACCOUNT_TYPE_GAMEMASTER and not player:hasFlag(PlayerFlag_CanTalkRedChannel) then
			if playerAccountType >= ACCOUNT_TYPE_TUTOR or player:hasFlag(PlayerFlag_TalkOrangeHelpChannel) then
				type = TALKTYPE_CHANNEL_O
			else
				type = TALKTYPE_CHANNEL_Y
			end
		end
	end
	return type
end

channel:register()
