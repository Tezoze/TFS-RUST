-- Makao (DialogueBuilder)
local npc = DialogueBuilder:new("Makao",
	{lookType = 143, lookHead = 114, lookBody = 132, lookLegs = 78, lookFeet = 116, lookAddons = 0})
npc:walkInterval(1500)
npc:greetMessage("Greetings, friend!")
npc:farewellMessage("Until we meet again!")
npc:walkawayMessage("Take care!")
npc:register()
