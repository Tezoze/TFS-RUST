local weapon = Weapon(WEAPON_AMMO)

-- TFS-shaped distance ammo: `onUseWeapon` owns primary + poison DoT
-- (`weapons.cpp` executeUseWeapon). Native DistanceAttack skips damage/specials
-- when `has_on_use` is set — same seam as burst_arrow.lua.
--
-- Outcomes: physical via COMBAT_FORMULA_SKILL; poison via ConditionDamage
-- (772 `DAMAGE_POISON_PERIODIC` / `SetTimer(SKILL_POISON, …)` — cycle from
-- `items.xml` poisondamagecycles=50, count/max 3 like envenom_rune).

local combat = Combat()
combat:setParameter(COMBAT_PARAM_TYPE, COMBAT_PHYSICALDAMAGE)
combat:setParameter(COMBAT_PARAM_DISTANCEEFFECT, CONST_ANI_POISONARROW)
combat:setParameter(COMBAT_PARAM_BLOCKARMOR, true)
combat:setFormula(COMBAT_FORMULA_SKILL, 0, 0, 1, 0)

local condition = Condition(CONDITION_POISON)
condition:setParameter(CONDITION_PARAM_CYCLE, 50)
condition:setParameter(CONDITION_PARAM_COUNT, 3)
condition:setParameter(CONDITION_PARAM_MAX_COUNT, 3)

function weapon.onUseWeapon(player, variant, hit)
	-- Miss: missile/ammo consume stay in Rust DistanceAttack; no damage/DoT.
	if not hit then
		return false
	end
	if not combat:execute(player, variant) then
		return false
	end
	local target = Creature(variant.number)
	if target then
		condition:setParameter(CONDITION_PARAM_OWNERGUID, player:getGuid())
		target:addCondition(condition)
	end
	return true
end

weapon:action("removecount")
weapon:id(2545)
weapon:register()
