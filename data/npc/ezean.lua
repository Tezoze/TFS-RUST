-- Ezean (ShopBuilder)
local npc = ShopBuilder:new("Ezean",
	{lookType = 339})
npc:walkInterval(0)
npc:addBuyable("label", 2599, 1)
npc:addBuyable("letter", 2597, 8)
npc:addBuyable("parcel", 2595, 15)
npc:register()
