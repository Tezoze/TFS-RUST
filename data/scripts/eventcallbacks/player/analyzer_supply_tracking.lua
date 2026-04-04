--[[
	Game Analyzer Supply Tracking System
	Tracks when consumable items are used
	Note: This tracks items when they're moved, as there's no onConsumeItem callback
	For proper tracking, you may need to integrate this into individual potion/rune scripts
]]

-- Track supply usage by detecting item removal patterns
-- This is a simplified version - for better tracking, modify individual potion/food scripts

-- Supply items to track
local supplyIds = {
	-- Health Potions
	[7618] = true,  -- health potion
	[7588] = true,  -- strong health potion
	[7591] = true,  -- great health potion
	[8473] = true,  -- ultimate health potion
	[23375] = true, -- supreme health potion
	
	-- Mana Potions  
	[7620] = true,  -- mana potion
	[7589] = true,  -- strong mana potion
	[7590] = true,  -- great mana potion
	[8472] = true,  -- ultimate mana potion
	[23373] = true, -- supreme mana potion
	
	-- Spirit Potions
	[8704] = true,  -- small spirit potion
	[7642] = true,  -- great spirit potion
	
	-- Runes (commonly used)
	[3147] = true,  -- blank rune
	[3155] = true,  -- sudden death rune
	[3161] = true,  -- avalanche rune
	[3164] = true,  -- stone shower rune
	[3165] = true,  -- thunderstorm rune
	[3166] = true,  -- great fireball rune
	[3188] = true,  -- heavy magic missile rune
	[3189] = true,  -- icicle rune
	[3190] = true,  -- explosion rune
	[3191] = true,  -- ultimate healing rune
	[3192] = true,  -- intense healing rune
}

-- DISABLED: Supply tracker disabled temporarily due to issues
-- Global function that can be called from potion/rune scripts
function trackSupplyUsage(player, itemId)
	if player and player:isPlayer() then
		-- DISABLED: player:sendSupplyUsed(itemId)
	end
end

print(">> Game Analyzer supply tracking DISABLED (temporarily disabled)")
print(">> trackSupplyUsage() function exists but calls are commented out")
print(">> Automatic supply tracking still works via server actions/weapons/spells")

