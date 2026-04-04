-- Black Bert (ShopBuilder)
local npc = ShopBuilder:new("Black Bert",
	{lookType = 151, lookBody = 38, lookLegs = 19, lookFeet = 76})
npc:greetMessage("Psst! Over here! You're the one Dorian mentioned, right? Want to see my {wares}?")
npc:farewellMessage("Keep it quiet, |PLAYERNAME|.")
npc:register()
