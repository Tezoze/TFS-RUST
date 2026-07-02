local spell = Spell(SPELL_INSTANT)

function spell.onCastSpell(creature, variant)
	return creature:conjureItem(spell, 0, 2545, 5)
end

spell:mana(130)
spell:level(16)
spell:soul(2)
spell:isAggressive(false)
spell:name("Poisoned Arrow")
spell:vocation("Paladin", "Royal Paladin")
spell:words("ex,evo, con, pox")
spell:register()