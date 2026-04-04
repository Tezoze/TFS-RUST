--[[
	Game Analyzer Impact Tracking System
	Provides helper functions for tracking damage and healing
	These should be called from spell scripts and combat handlers
]]

-- Constants for analyzer types (match client-side values)
ANALYZER_DAMAGE_DEALT = 1
ANALYZER_DAMAGE_RECEIVED = 2
ANALYZER_HEAL = 3

-- DISABLED: Impact tracker functions disabled due to missing server implementation
-- These would cause errors since sendImpactTracker method doesn't exist on server

-- Global function to track healing (call from healing spells)
function trackHealing(player, amount, element)
	-- DISABLED: player:sendImpactTracker(ANALYZER_HEAL, amount, element or 0, "")
end

-- Global function to track damage dealt (call from damage spells)
function trackDamageDealt(player, target, amount, combatType)
	-- DISABLED: local targetName = target and target:getName() or ""
	-- DISABLED: player:sendImpactTracker(ANALYZER_DAMAGE_DEALT, amount, combatType or 0, targetName)
end

-- Global function to track damage received (call from damage handlers)
function trackDamageReceived(player, attacker, amount, combatType)
	-- DISABLED: local attackerName = attacker and attacker:getName() or ""
	-- DISABLED: player:sendImpactTracker(ANALYZER_DAMAGE_RECEIVED, amount, combatType or 0, attackerName)
end

print(">> Game Analyzer impact tracking DISABLED (missing server implementation)")
print(">> Impact tracker functions are commented out to prevent errors")

