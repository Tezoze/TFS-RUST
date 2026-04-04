local combat = Combat()
combat:setParameter(COMBAT_PARAM_EFFECT, CONST_ME_SOUND_RED)
combat:setArea(createCombatArea(AREA_SQUAREWAVE6))

function onCastSpell(creature, variant)
	local reduction = math.random(60, 95)
	local parameters = {
		{key = CONDITION_PARAM_TICKS, value = 4 * 1000},
		{key = CONDITION_PARAM_SKILL_SHIELDPERCENT, value = reduction}
	}

	for _, target in ipairs(combat:getTargets(creature, variant)) do
		target:addAttributeCondition(parameters)
	end
	return true
end
