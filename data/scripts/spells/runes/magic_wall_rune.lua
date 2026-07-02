local spell = Spell(SPELL_INSTANT)

spell:mana(750)
spell:level(32)
spell:soul(5)
spell:isAggressive(false)
spell:isPremium(true)
spell:name("Magic Wall")
spell:vocation("Sorcerer", "Master Sorcerer")
spell:words("ad,evo, grav, tera")

function spell.onCastSpell(creature, variant)
	return creature:conjureItem(750, 2260, 2293, 3)
end

spell:register()

local combat = Combat()
combat:setParameter(COMBAT_PARAM_DISTANCEEFFECT, CONST_ANI_ENERGY)
combat:setParameter(COMBAT_PARAM_CREATEITEM, ITEM_MAGICWALL)
combat:setParameter(COMBAT_PARAM_AGGRESSIVE, true)

local rune = Spell(SPELL_RUNE)

function rune.onCastSpell(creature, variant)
	if variant.pos then
		local tile = Tile(variant.pos)
		if tile:hasProperty(CONST_PROP_BLOCKSOLID) then
			creature:sendTextMessage(MESSAGE_STATUS_SMALL, Game.getReturnMessage(RETURNVALUE_NOTENOUGHROOM))
			return false
		end
	end
	return combat:execute(creature, variant)
end

rune:runeMagicLevel(9)
rune:runeId(2293)
rune:charges(3)
rune:allowFarUse(true)
rune:checkFloor(true)
rune:isBlocking(true, true)
rune:isAggressive(true)
rune:register()
