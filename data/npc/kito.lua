-- Kito (DialogueBuilder)
local npc = DialogueBuilder:new("Kito",
	{lookType = 143, lookHead = 115, lookBody = 98, lookLegs = 97, lookFeet = 116, lookAddons = 0})
npc:walkInterval(1500)
npc:greetMessage("Hello there, traveler!")
npc:farewellMessage("Safe travels!")
npc:walkawayMessage("Goodbye!")
npc:register()
