-- Dankwart (ShopBuilder)
local npc = ShopBuilder:new("Dankwart",
	{lookType = 128, lookHead = 39, lookBody = 58, lookLegs = 58, lookFeet = 115})
npc:addBuyable("bread", 2689, 4)
npc:addBuyable("cheese", 2696, 6)
npc:addBuyable("ham", 2671, 8)
npc:addBuyable("meat", 2666, 5)
npc:addBuyable("mug of mead", 2012, 5)
npc:register()
