function onStepIn(creature, item, position, fromPosition)
	local player = creature:getPlayer()
	if not player then
		return true
	end

	-- Check if player has the Honour The Dead task active
	if player:getStorageValue(Storage.AnUneasyAllianceTasks.HonourDead) == 1 then
		player:setStorageValue(Storage.AnUneasyAllianceTasks.HonourDead, 2)
		player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "You meditate on the grave of the fallen orc warrior, honouring his memory and contemplating the fragility of life.")
		position:sendMagicEffect(CONST_ME_MAGIC_BLUE)
		player:say("*meditates*", TALKTYPE_MONSTER_SAY)
	end

	return true
end
