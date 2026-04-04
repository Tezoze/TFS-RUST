-- Gnomailion (ShopBuilder)
local npc = ShopBuilder:new("Gnomailion",
	{lookType = 493, lookHead = 94, lookBody = 88, lookLegs = 88, lookFeet = 114})
npc:addBuyable("label", 2599, 1)
npc:addBuyable("letter", 2597, 8)
npc:addBuyable("parcel", 2595, 15)
npc:register()
