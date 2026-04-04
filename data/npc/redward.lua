-- Redward (ShopBuilder)
local npc = ShopBuilder:new("Redward",
	{lookType = 237, lookHead = 20, lookBody = 39, lookLegs = 45, lookFeet = 7})
npc:addBuyable("label", 2599, 1)
npc:addBuyable("letter", 2597, 10)
npc:addBuyable("parcel", 2595, 15)
npc:register()
