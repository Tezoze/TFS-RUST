-- Azalea (NpcBuilder)
local npc = NpcBuilder:new("Azalea",
	{lookType = 433, lookBody = 24, lookLegs = 27, lookFeet = 40, lookAddons = 3})
npc:greetMessage("Welcome, young |PLAYERNAME|! If you are heavily wounded or poisoned, I can {heal} you for free.")
npc:farewellMessage("May the gods bless you, |PLAYERNAME|!")
npc:walkawayMessage("Remember: If you are heavily wounded or poisoned, I can heal you for free.")
npc:register()
