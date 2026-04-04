-- Lucius (DialogueBuilder)
local npc = DialogueBuilder:new("Lucius",
	{lookType = 325, lookHead = 76, lookBody = 79, lookLegs = 117, lookFeet = 114, lookAddons = 3})
npc:greetMessage("Want to go back to {Temple} of Light for 50 gold? Just ask me.")
npc:farewellMessage("Good bye.")
npc:walkawayMessage("Good bye then.")
npc:register()
