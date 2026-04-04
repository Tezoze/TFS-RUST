-- Gnomejam (ShopBuilder)
local npc = ShopBuilder:new("Gnomejam",
	{lookType = 493, lookHead = 3, lookBody = 58, lookLegs = 58, lookFeet = 95})
npc:addBuyable("Bread", 2689, 4)
npc:addBuyable("Cheese", 2696, 6)
npc:addBuyable("Ham", 2671, 8)
npc:addBuyable("Meat", 2666, 5)
npc:register()
