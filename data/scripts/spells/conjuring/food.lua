local foods = {
	3577, -- meat
	3582, -- ham
	3592, -- grape
	3585, -- apple
	3600, -- bread
	3601, -- roll
	3607  -- cheese
}

local spell = Spell(SPELL_INSTANT)

function spell.onCastSpell(creature, variant)
	if math.random(0, 1) == 1 then
		creature:addItem(foods[math.random(#foods)])
	end

	creature:addItem(foods[math.random(#foods)])
	creature:getPosition():sendMagicEffect(CONST_ME_MAGIC_GREEN)
	return true
end

spell:mana(120)
spell:level(14)
spell:isAggressive(false)
spell:name("Food")
spell:vocation("Druid", "Elder Druid", "Paladin", "Royal Paladin")
spell:words("ex,evo, pan")
spell:register()