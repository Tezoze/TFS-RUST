-- Rock With A Soft Spot (NpcBuilder)
local npc = NpcBuilder:new("Rock With A Soft Spot",
	{lookTypeEx = 14896})
npc:walkInterval(0)
npc:greetMessage("Welcome, this is the {Gray Beach temple}, |PLAYERNAME|. Whether you are wounded, poisoned - or wait, don")
npc:farewellMessage("Goodbye then, |PLAYERNAME|!")
npc:walkawayMessage("Only the best for you.")
npc:register()
