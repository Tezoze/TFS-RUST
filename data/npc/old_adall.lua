-- Old Adall (TravelBuilder)
local npc = TravelBuilder:new("Old Adall",
	{lookType = 130, lookHead = 95, lookBody = 26, lookLegs = 115, lookFeet = 76})
npc:addDestination("East", {x = 32679, y = 32777, z = 7}, 7)
npc:addDestination("West", {x = 32558, y = 32780, z = 7}, 7)
npc:register()
