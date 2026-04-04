-- Soilance (ShopBuilder)
local npc = ShopBuilder:new("Soilance",
	{lookType = 128, lookHead = 79, lookBody = 48, lookLegs = 57, lookFeet = 76})
npc:addBuyable("ham", 2671, 5)
npc:addBuyable("meat", 2666, 3)
npc:register()
