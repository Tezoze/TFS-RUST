-- Tokel (ShopBuilder)
local npc = ShopBuilder:new("Tokel",
	{lookType = 128, lookHead = 78, lookBody = 96, lookLegs = 30, lookFeet = 114})
npc:addBuyable("bread", 2689, 3)
npc:addBuyable("cheese", 2696, 5)
npc:addBuyable("ham", 2671, 8)
npc:addBuyable("meat", 2666, 5)
npc:register()
