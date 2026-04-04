-- Tatak (DialogueBuilder)
local npc = DialogueBuilder:new("Tatak",
	{lookType = 143, lookHead = 77, lookBody = 101, lookLegs = 96, lookFeet = 97, lookAddons = 0})
npc:walkInterval(1500)
npc:greetMessage("Ah, a visitor! Welcome!")
npc:farewellMessage("Safe journey, friend!")
npc:walkawayMessage("Be well!")
npc:register()
