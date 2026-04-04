-- Harlow (TravelBuilder)
local npc = TravelBuilder:new("Harlow",
	{lookType = 151, lookHead = 116, lookBody = 77, lookLegs = 113})
npc:farewellMessage("Good bye.")
npc:walkawayMessage("Good bye then.")
npc:addDestination("Vengoth", {x = 32858, y = 31549, z = 7}, 200)
npc:addVoice("Passages to Vengoth.")
npc:greetMessage("Welcome on board, |PLAYERNAME|. Where can I {sail} you today?")
npc:register()
