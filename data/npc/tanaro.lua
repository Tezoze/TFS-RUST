-- Tanaro (ShopBuilder)
local npc = ShopBuilder:new("Tanaro",
	{lookType = 144, lookHead = 113, lookLegs = 97, lookFeet = 115, lookAddons = 1})
npc:addBuyable("bottle of water", 2007, 2)
npc:addBuyable("green flask of wine", 2009, 3)
npc:addBuyable("wine", 2006, 10)
npc:register()
