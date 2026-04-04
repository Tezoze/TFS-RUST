-- Nielson (TravelBuilder)
local npc = TravelBuilder:new("Nielson",
	{lookType = 129, lookHead = 114, lookBody = 113, lookLegs = 68, lookFeet = 67})
npc:addDestination("Vega", {x = 32020, y = 31692, z = 7}, 20)
npc:addDestination("Senja", {x = 32128, y = 31664, z = 7}, 20)
npc:addDestination("Folda", {x = 32046, y = 31578, z = 7}, 20)
npc:register()
