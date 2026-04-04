local area = createCombatArea({
	{1, 1, 1},
	{1, 3, 1},
	{1, 1, 1}
})

local combat = Combat()
combat:setParameter(COMBAT_PARAM_TYPE, COMBAT_PHYSICALDAMAGE)
combat:setParameter(COMBAT_PARAM_EFFECT, CONST_ME_EXPLOSIONAREA)
combat:setParameter(COMBAT_PARAM_DISTANCEEFFECT, CONST_ANI_BURSTARROW)
combat:setParameter(COMBAT_PARAM_BLOCKARMOR, true)
combat:setFormula(COMBAT_FORMULA_SKILL, 0, 0, 1, 0)
combat:setArea(area)

function onUseWeapon(player, variant)
	if player:getSkull() == SKULL_BLACK then
		return false
	end

	local result = combat:execute(player, variant)

	-- Check if player is a distance vocation (Paladin or Royal Paladin)
	local isDistanceVocation = (player:getVocation():getId() == 3 or player:getVocation():getId() == 7)

	if isDistanceVocation then
		-- Get the target creature
		local target = nil
		if variant.type == VARIANT_NUMBER then
			target = Creature(variant.number)
		elseif variant.type == VARIANT_POSITION then
			local tile = Tile(variant.position)
			if tile then
				target = tile:getTopCreature()
			end
		end

		-- Check if target is a servant golem
		local isServantGolem = target and target:getName():lower() == "servant golem"

		-- Always give distance skill training for burst arrows when:
		-- 1. Combat succeeds (normal case), OR
		-- 2. Targeting a servant golem (immune to damage but should still train)
		if result or isServantGolem then
			-- Custom training system that bypasses blood hit requirements
			-- Apply the skill rate from config.lua
			local skillRate = configManager.getNumber(configKeys.RATE_SKILL)
			local skillTries = math.floor(2 * skillRate) -- Give 2 tries per shot (equivalent to a good hit), multiplied by skill rate
			player:addSkillTries(SKILL_DISTANCE, skillTries)
		end
	end

	return result
end
