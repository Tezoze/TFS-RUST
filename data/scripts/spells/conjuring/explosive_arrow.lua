local spell = Spell(SPELL_INSTANT)

function spell.onCastSpell(creature, variant)
	return creature:conjureItem(spell, 0, 2546, 3)
end

spell:mana(290)
spell:level(25)
spell:soul(3)
spell:isAggressive(false)
spell:isPremium(true)
spell:name("Explosive Arrow")
spell:vocation("Paladin", "Royal Paladin")
spell:words("ex,evo, con, flam")
spell:register()