local statues = {
	[18488] = SKILL_SWORD,  -- Melee Fighting
	[18489] = SKILL_SWORD,  -- Melee Fighting (was Axe)
	[18490] = SKILL_SWORD,  -- Melee Fighting (was Club)
	[18491] = SKILL_DISTANCE,
	[18492] = SKILL_MAGLEVEL
}

function onUse(player, item, fromPosition, target, toPosition, isHotkey)
	local skill = statues[item:getId()]
	if not player:isPremium() then
		player:sendCancelMessage(RETURNVALUE_YOUNEEDPREMIUMACCOUNT)
		return true
	end

	if player:isPzLocked() then
		return false
	end

	player:setOfflineTrainingSkill(skill)
	player:remove()
	return true
end
