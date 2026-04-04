-- Satsu (ShopBuilder)
local npc = ShopBuilder:new("Satsu",
	{lookType = 158, lookHead = 78, lookBody = 96, lookLegs = 118, lookFeet = 96})
npc:addSellable("cocktail glass", 10150, 50)
npc:addBuyable("cocktail glass of fruit juice", 10150, 52)
npc:addBuyable("wine", 2006, 10)
npc:register()
