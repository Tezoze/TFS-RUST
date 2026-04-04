-- A Grumpy Cyclops (DialogueBuilder)
local npc = DialogueBuilder:new("A Grumpy Cyclops",
	{lookType = 277, lookHead = 58, lookBody = 43, lookLegs = 38, lookFeet = 76, lookAddons = 0})
npc:greetMessage("Hrmpf! What do you want, tiny human?")
npc:farewellMessage("Good riddance!")
npc:walkawayMessage("Typical human...")
npc:register()
