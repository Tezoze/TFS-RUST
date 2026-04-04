-- Namasa (DialogueBuilder)
local npc = DialogueBuilder:new("Namasa",
	{lookType = 147, lookHead = 78, lookBody = 61, lookLegs = 78, lookFeet = 114, lookAddons = 0})
npc:walkInterval(1500)
npc:greetMessage("Hello, welcome to our village!")
npc:farewellMessage("May the gods watch over you!")
npc:walkawayMessage("Farewell!")
npc:register()
