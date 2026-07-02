local spell = Spell(SPELL_INSTANT)

function spell.onCastSpell(creature, variant)
	return creature:conjureItem(spell, 0, 2543, 5)
end

spell:mana(140)
spell:level(17)
spell:soul(2)
spell:isAggressive(false)
spell:name("Conjure Bolt")
spell:vocation("Paladin", "Royal Paladin")
spell:words("ex,evo, con, mort")
spell:register()