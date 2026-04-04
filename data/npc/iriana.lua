-- Iriana (ShopBuilder)
local npc = ShopBuilder:new("Iriana",
	{lookType = 264, lookHead = 78, lookBody = 116, lookLegs = 95, lookFeet = 121})
npc:addBuyable("backpack", 1988, 10)
npc:addBuyable("bag", 1987, 5)
npc:addBuyable("fishing rod", 2580, 150)
npc:addBuyable("worm", 3976, 1)
npc:register()
