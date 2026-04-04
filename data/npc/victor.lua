-- Victor (ShopBuilder)
local npc = ShopBuilder:new("Victor",
	{lookType = 134, lookHead = 21, lookBody = 78, lookLegs = 38, lookFeet = 95, lookAddons = 3})
npc:addBuyable("rusty", 24181, 5000)
npc:addBuyable("stone", 24173, 5000)
npc:register()
