-- Loui (DialogueBuilder)
local npc = DialogueBuilder:new("Loui",
	{lookType = 57})
npc:greetMessage("BEWARE! Beware of that {hole}!")
npc:farewellMessage("May the gods protect you! And stay away from that hole!")
npc:walkawayMessage("STAY AWAY FROM THAT HOLE!")
npc:register()
