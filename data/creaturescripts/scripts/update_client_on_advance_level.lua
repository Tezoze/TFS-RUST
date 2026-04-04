function onAdvance(player, skill, oldLevel, newLevel)
	if skill ~= SKILL_LEVEL then
		return true
	end

	-- Note: Client experience display updates are not implemented in this TFS version
	return true
end
