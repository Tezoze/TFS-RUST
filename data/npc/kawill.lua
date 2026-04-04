-- Kawill (DialogueBuilder)
local npc = DialogueBuilder:new("Kawill",
	{lookType = 66})
npc:greetMessage("Welcome |PLAYERNAME|! May earth protect you!")
npc:farewellMessage("Earth under your feet, |PLAYERNAME|!")
npc:walkawayMessage("Earth under your feet, pilgrim. What brings you here?")
npc:register()
