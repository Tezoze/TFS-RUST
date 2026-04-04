-- Yulas (ShopBuilder)
local npc = ShopBuilder:new("Yulas",
	{lookType = 128, lookHead = 58, lookBody = 43, lookLegs = 38, lookFeet = 76})
npc:addBuyable("big table kit", 3911, 30)
npc:addBuyable("small table kit", 3908, 20)
npc:addBuyable("trophy stand", 7936, 50)
npc:register()
