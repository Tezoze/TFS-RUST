-- Track hits per spell cast
local spellHits = {}

local combat = Combat()
combat:setParameter(COMBAT_PARAM_TYPE, COMBAT_FIREDAMAGE)
combat:setParameter(COMBAT_PARAM_EFFECT, CONST_ME_FIREAREA)
combat:setParameter(COMBAT_PARAM_DISTANCEEFFECT, CONST_ANI_FIRE)
combat:setArea(createCombatArea(AREA_CIRCLE3X3))

function onGetFormulaValues(player, level, magicLevel)
	local min = (level / 5) + (magicLevel * 1.2) + 7
	local max = (level / 5) + (magicLevel * 2.85) + 16
	return -min, -max
end

function onTargetCreature(creature, target)
	-- Initialize hit counter for this cast
	local casterId = creature:getId()
	if not spellHits[casterId] then
		spellHits[casterId] = 0
	end
	
	-- Increment hit count
	spellHits[casterId] = spellHits[casterId] + 1
	local hitNumber = spellHits[casterId]
	
	-- Calculate damage multiplier based on hit order (3% reduction per target)
	local multiplier = 1.0
	if hitNumber == 1 then
		multiplier = 1.00
	elseif hitNumber == 2 then
		multiplier = 0.97
	elseif hitNumber == 3 then
		multiplier = 0.94
	elseif hitNumber == 4 then
		multiplier = 0.91
	elseif hitNumber == 5 then
		multiplier = 0.88
	elseif hitNumber == 6 then
		multiplier = 0.85
	else -- 7+
		multiplier = 0.85
	end
	
	-- Get player and calculate base damage
	local player = creature:getPlayer()
	if not player then
		return true
	end
	
	local level = player:getLevel()
	local magicLevel = player:getMagicLevel()
	local min = (level / 5) + (magicLevel * 1.2) + 7
	local max = (level / 5) + (magicLevel * 2.85) + 16
	
	-- Apply multiplier and randomize damage
	local damage = math.random(math.floor(min * multiplier), math.floor(max * multiplier))
	
	-- Apply damage to target
	doTargetCombatHealth(creature, target, COMBAT_FIREDAMAGE, -damage, -damage, CONST_ME_FIREAREA)
	
	return true
end

combat:setCallback(CALLBACK_PARAM_TARGETCREATURE, "onTargetCreature")

function onCastSpell(creature, variant, isHotkey)
	-- Reset hit counter before casting
	local casterId = creature:getId()
	spellHits[casterId] = 0
	
	local result = combat:execute(creature, variant)
	
	-- Clean up after a short delay
	addEvent(function()
		spellHits[casterId] = nil
	end, 100)
	
	return result
end
