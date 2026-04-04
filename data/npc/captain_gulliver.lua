-- Captain Gulliver (TravelBuilder)
local npc = TravelBuilder:new("Captain Gulliver",
	{lookType = 472, lookBody = 57, lookLegs = 20, lookFeet = 39})
npc:addDestination("Thais", {x = 32311, y = 32210, z = 6}, 150)
npc:addDestination("Krailos", {x = 33493, y = 31712, z = 6}, 180)
npc:addVoice("Passages to Thais and Krailos! Visit the strange lands!")
npc:greetMessage("Welcome on board, Sir |PLAYERNAME|. Where can I {sail} you today?")
npc:farewellMessage("Good bye. Recommend us if you were satisfied with our service.")
npc:register()
