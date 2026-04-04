-- Zedrulon The Fallen (NpcBuilder)
local npc = NpcBuilder:new("Zedrulon The Fallen",
	{lookType = 289, lookHead = 2, lookBody = 114, lookLegs = 75, lookFeet = 97})
npc:greetMessage("Welcome, young |PLAYERNAME|! If you are heavily wounded or poisoned, I can {heal} you for free.")
npc:farewellMessage("May the gods bless you, |PLAYERNAME|!")
npc:walkawayMessage("Remember: If you are heavily wounded or poisoned, I can heal you for free.")
npc:register()
