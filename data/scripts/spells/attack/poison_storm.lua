local combat = Combat()
combat:setParameter(COMBAT_PARAM_TYPE, COMBAT_EARTHDAMAGE)
combat:setParameter(COMBAT_PARAM_EFFECT, CONST_ME_GREEN_RINGS)
combat:setParameter(COMBAT_PARAM_AGGRESSIVE, true)
combat:setParameter(COMBAT_PARAM_FORCEONTARGETEVENT, true)
combat:setArea(createCombatArea(AREA_CIRCLE6X6))

local condition = Condition(CONDITION_POISON)

local spell = Spell(SPELL_INSTANT)

function spell.onCastSpell(creature, variant)
	-- Area effect on every disc tile (`EFFECT_POISON`, `magic.cc:496`). Damage stays 0;
	-- the poison condition below is the hit. `execute` also refuses secure-mode casts.
	if not combat:execute(creature, variant) then
		return false
	end

	local blockPz = false
	local cycles = -creature:computeDamage(200, 50)
	condition:setParameter(CONDITION_PARAM_CYCLE, cycles)
	condition:setParameter(CONDITION_PARAM_COUNT, 3)
	condition:setParameter(CONDITION_PARAM_MAX_COUNT, 3)
	condition:setParameter(CONDITION_PARAM_OWNERGUID, creature:getGuid())
	local casterId = creature:getId()
	for _, target in ipairs(combat:getTargets(creature, variant)) do
		-- `TDamageImpact` does not hit the actor (`magic.cc:143`).
		if target:getId() ~= casterId then
			if target:isPlayer() then
				blockPz = true
			end
			target:addCondition(condition)
		end
	end

	if blockPz then
		creature:setInFight(true)
	end

	return true
end

spell:mana(600)
spell:level(50)
spell:isPremium(true)
spell:isAggressive(true)
spell:name("Poison Storm")
spell:vocation("Druid", "Elder Druid")
spell:words("ex,evo, gran, mas, pox")
spell:register()