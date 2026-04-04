-- Ray (ShopBuilder)
local npc = ShopBuilder:new("Ray",
	{lookType = 128, lookHead = 19, lookBody = 115, lookLegs = 126, lookFeet = 58})
npc:addBuyable("label", 2599, 1)
npc:addBuyable("letter", 2597, 8)
npc:addBuyable("parcel", 2595, 15)
npc:register()
