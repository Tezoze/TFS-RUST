local spell = Spell(SPELL_INSTANT)

spell:mana(400)
spell:level(21)
spell:soul(2)
spell:isAggressive(false)
spell:isPremium(true)
spell:name("Envenom")
spell:vocation("Druid", "Elder Druid")
spell:words("ad,evo, res, pox")

function spell.onCastSpell(creature, variant)
	return creature:conjureItem(400, 2260, 2292, 1)
end

spell:register()

local combat = Combat()
combat:setParameter(COMBAT_PARAM_TYPE, COMBAT_EARTHDAMAGE)
combat:setParameter(COMBAT_PARAM_EFFECT, CONST_ME_HITBYPOISON)
combat:setParameter(COMBAT_PARAM_DISTANCEEFFECT, CONST_ANI_ENERGY)
combat:setParameter(COMBAT_PARAM_AGGRESSIVE, true)

local condition = Condition(CONDITION_POISON)

local rune = Spell(SPELL_RUNE)

function rune.onCastSpell(creature, variant)
	local cycles = -creature:computeDamage(70, 20, true)
	condition:setParameter(CONDITION_PARAM_CYCLE, cycles)
	condition:setParameter(CONDITION_PARAM_COUNT, 3)
	condition:setParameter(CONDITION_PARAM_MAX_COUNT, 3)
	condition:setParameter(CONDITION_PARAM_OWNERGUID, creature:getGuid())
	local target = Creature(variant.number)
	if target then
		target:addCondition(condition)
	end
	return combat:execute(creature, variant)
end

rune:runeMagicLevel(4)
rune:runeId(2292)
rune:charges(1)
rune:allowFarUse(true)
rune:blockWalls(true)
rune:checkFloor(true)
rune:needTarget(true)
rune:register()
