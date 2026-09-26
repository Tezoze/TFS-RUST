local spell = Spell(SPELL_INSTANT)

function spell.onCastSpell(creature, variant)
	return creature:conjureItem(spell, 3289, 3321, 1)
end

spell:mana(80)
spell:level(41)
spell:soul(0)
spell:isPremium(true)
spell:isAggressive(false)
spell:name("Enchant Staff")
spell:vocation("Master Sorcerer")
spell:words("ex,eta, vis")
spell:register()