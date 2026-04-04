-- Wally (ShopBuilder)
local npc = ShopBuilder:new("Wally",
	{lookType = 129, lookHead = 96, lookBody = 113, lookLegs = 95, lookFeet = 115})
npc:addBuyable("label", 2599, 1)
npc:addBuyable("letter", 2597, 8)
npc:addBuyable("parcel", 2595, 15)
npc:register()
