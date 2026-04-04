local event = Event()

function event.onShareExperience(party, multiplierPercent, rawExp)
	-- The multiplier is already calculated in the main party script as a percentage (120 = 120%)
	-- This callback can be used for additional modifications if needed
	-- For now, just return the multiplier as-is
	return multiplierPercent
end

event:register()
