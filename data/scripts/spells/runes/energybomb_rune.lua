local spell = Spell(SPELL_INSTANT)

spell:mana(880)
spell:level(37)
spell:soul(5)
spell:isAggressive(false)
spell:isPremium(true)
spell:name("Energybomb")
spell:vocation("Sorcerer", "Master Sorcerer")
spell:words("ad,evo, mas, vis")

function spell.onCastSpell(creature, variant)
	return creature:conjureItem(880, 2260, 2262, 2)
end

spell:register()

local combat = Combat()
combat:setParameter(COMBAT_PARAM_TYPE, COMBAT_ENERGYDAMAGE)
combat:setParameter(COMBAT_PARAM_DISTANCEEFFECT, CONST_ANI_ENERGY)
combat:setParameter(COMBAT_PARAM_CREATEITEM, ITEM_ENERGYFIELD_PVP)
combat:setParameter(COMBAT_PARAM_AGGRESSIVE, true)
combat:setArea(createCombatArea(AREA_SQUARE1X1))

local rune = Spell(SPELL_RUNE)

function rune.onCastSpell(creature, variant)
	return combat:execute(creature, variant, false)
end

rune:runeMagicLevel(10)
rune:runeId(2262)
rune:charges(2)
rune:allowFarUse(true)
rune:checkFloor(true)
rune:isBlocking(true)
rune:isPzLock(true)
rune:register()