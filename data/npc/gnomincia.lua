-- Gnomincia (ShopBuilder)
local npc = ShopBuilder:new("Gnomincia",
	{lookType = 507, lookHead = 79, lookBody = 94, lookLegs = 94, lookFeet = 52})
npc:addBuyable("teleport crystal", 18457, 150)
npc:register()
