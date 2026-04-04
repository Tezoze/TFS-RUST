-- James (ShopBuilder)
local npc = ShopBuilder:new("James",
	{lookType = 128, lookHead = 115, lookBody = 60, lookLegs = 44, lookFeet = 118})
npc:addBuyable("bread", 2689, 3)
npc:addBuyable("cheese", 2696, 5)
npc:addBuyable("ham", 2671, 8)
npc:addBuyable("meat", 2666, 5)
npc:addBuyable("red apple", 2674, 3)
npc:register()
