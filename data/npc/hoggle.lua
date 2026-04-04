-- Hoggle (DialogueBuilder)
local npc = DialogueBuilder:new("Hoggle",
	{lookType = 128, lookHead = 21, lookBody = 46, lookLegs = 88, lookFeet = 94})
npc:greetMessage("Welcome to my humble home!")
npc:farewellMessage("Good bye.")
npc:walkawayMessage("Good bye.")
npc:addVoice("Oh, this misery...")
npc:register()
