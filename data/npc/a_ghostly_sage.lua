-- A Ghostly Sage (DialogueBuilder)
local npc = DialogueBuilder:new("A Ghostly Sage",
	{lookType = 130, lookHead = 9, lookBody = 85, lookLegs = 9, lookFeet = 85, lookAddons = 1})
npc:walkInterval(0)
npc:greetMessage("Ah, I feel a mortal walks these ancient halls again. Pardon me, I barely notice you. I am so lost in my thoughts.")
npc:farewellMessage("Good bye.")
npc:walkawayMessage("Farewell then.")
npc:register()
