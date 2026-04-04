-- Ashari (DialogueBuilder)
local npc = DialogueBuilder:new("Ashari",
	{lookType = 159, lookHead = 38, lookBody = 117, lookLegs = 117, lookFeet = 116})
npc:greetMessage("Hello, stranger! These caves must seem strange to you. I wonder what brings you here... maybe you are interested in some work? There are several tasks I could need a hand with.")
npc:farewellMessage("Bye!")
npc:walkawayMessage("Bye!")
npc:register()
