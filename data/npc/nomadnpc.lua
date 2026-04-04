-- Nomad (NpcBuilder)
local npc = NpcBuilder:new("Nomad",
	{lookType = 143, lookHead = 115, lookBody = 116, lookLegs = 59, lookFeet = 116})
npc:greetMessage("Welcome, noble |PLAYERNAME|")
npc:farewellMessage("Good Bye, noble |PLAYERNAME|")
npc:walkawayMessage("Good Bye, noble |PLAYERNAME|")
npc:register()
