-- Auron (ShopBuilder)
local npc = ShopBuilder:new("Auron",
	{lookType = 508})
npc:addBuyable("label", 2599, 1)
npc:addBuyable("letter", 2597, 8)
npc:addBuyable("parcel", 2595, 15)
npc:register()
