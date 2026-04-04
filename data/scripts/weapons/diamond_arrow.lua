local area = createCombatArea({
	{ 0, 1, 1, 1, 0 },
	{ 1, 1, 1, 1, 1 },
	{ 1, 1, 3, 1, 1 },
	{ 1, 1, 1, 1, 1 },
	{ 0, 1, 1, 1, 0 },
})

local combat = Combat()
combat:setParameter(COMBAT_PARAM_TYPE, COMBAT_PHYSICALDAMAGE)
combat:setParameter(COMBAT_PARAM_EFFECT, CONST_ME_ENERGYHIT)
combat:setParameter(COMBAT_PARAM_DISTANCEEFFECT, CONST_ANI_FLASHARROW)
combat:setParameter(COMBAT_PARAM_BLOCKARMOR, true)

function onGetFormulaValues(player, skill, attack, factor)
	local distanceSkill = player:getEffectiveSkillLevel(SKILL_DISTANCE)
	local min = (player:getLevel() / 5)
	local max = (0.09 * factor) * distanceSkill * attack + (player:getLevel() / 5)
	return -min, -max
end

combat:setCallback(CALLBACK_PARAM_SKILLVALUE, "onGetFormulaValues")
combat:setArea(area)

local diamondArrow = Weapon(WEAPON_AMMO)

function diamondArrow.onUseWeapon(player, variant)
	if player:getSkull() == SKULL_BLACK then
		return false
	end
	return combat:execute(player, variant)
end

diamondArrow:id(28413)
diamondArrow:level(100)
diamondArrow:attack(37)
diamondArrow:action("removecount")
diamondArrow:ammoType("arrow")
diamondArrow:shootType(CONST_ANI_FLASHARROW)
diamondArrow:maxHitChance(100)
diamondArrow:wieldUnproperly(true)
diamondArrow:register()
