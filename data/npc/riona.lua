-- Riona (ShopBuilder)
local npc = ShopBuilder:new("Riona",
	{lookType = 138, lookHead = 57, lookBody = 59, lookLegs = 40, lookFeet = 76})
npc:addBuyable("backpack", 1988, 20)
npc:addBuyable("pick", 2553, 10)
npc:addBuyable("rope", 2120, 50)
npc:addBuyable("shovel", 2554, 20)
npc:register()
