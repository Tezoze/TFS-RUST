-- Stan (ShopBuilder)
local npc = ShopBuilder:new("Stan",
	{lookType = 12, lookHead = 94, lookBody = 128, lookLegs = 81, lookFeet = 128})
npc:addBuyable("bag", 7737, 500)
npc:addBuyable("bag", 7739, 1500)
npc:addBuyable("bag", 9076, 1000)
npc:addBuyable("party hat", 6578, 600)
npc:register()
