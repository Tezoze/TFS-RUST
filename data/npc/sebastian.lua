-- Sebastian (TravelBuilder)
local npc = TravelBuilder:new("Sebastian",
	{lookType = 151, lookHead = 57, lookBody = 52, lookLegs = 109, lookFeet = 115, lookAddons = 3})
npc:greetMessage("Greetings, daring adventurer. If you need a {passage}, let me know.")
npc:farewellMessage("Good bye.")
npc:walkawayMessage("Oh well.")
npc:addDestination("Liberty Bay", {x = 32316, y = 32702, z = 7}, 50)
npc:addDestination("Nargor", {x = 32024, y = 32813, z = 7}, 50)
npc:register()
