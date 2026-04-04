-- Fyodor (ShopBuilder)
local npc = ShopBuilder:new("Fyodor",
	{lookType = 132, lookHead = 40, lookBody = 114, lookLegs = 60, lookFeet = 78, lookAddons = 1})
npc:addBuyable("label", 2599, 1)
npc:addBuyable("letter", 2597, 8)
npc:addBuyable("parcel", 2595, 15)
npc:register()
