local spell = Spell(SPELL_INSTANT)

function spell.onCastSpell(creature, variant)
	return creature:conjureItem(spell, 0, 2544, 10)
end

spell:mana(100)
spell:level(13)
spell:soul(1)
spell:isAggressive(false)
spell:name("Conjure Arrow")
spell:vocation("Paladin", "Royal Paladin")
spell:words("ex,evo, con")
spell:register()