local spell = Spell(SPELL_INSTANT)

function spell.onCastSpell(creature, variant)
	return creature:conjureItem(spell, 0, 2547, 1)
end

spell:mana(800)
spell:level(59)
spell:soul(3)
spell:isPremium(true)
spell:isAggressive(false)
spell:name("Conjure Power Bolt")
spell:vocation("Royal Paladin")
spell:words("ex,evo, con, vis")
spell:register()