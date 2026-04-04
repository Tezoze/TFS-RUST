-- Quero (ShopBuilder)
local npc = ShopBuilder:new("Quero",
	{lookType = 128, lookHead = 55, lookBody = 30, lookLegs = 24, lookFeet = 115})
npc:addBuyable("drum", 2073, 140)
npc:addBuyable("lute", 2072, 195)
npc:addBuyable("lyre", 2071, 120)
npc:addBuyable("simple fanfare", 2075, 150)
npc:register()
