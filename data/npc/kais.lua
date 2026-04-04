-- Kais (NpcBuilder)
local npc = NpcBuilder:new("Kais",
	{lookType = 103})
npc:greetMessage("Welcome, noble |PLAYERNAME|")
npc:farewellMessage("Good Bye, noble |PLAYERNAME|")
npc:walkawayMessage("Good Bye, noble |PLAYERNAME|")
npc:register()
