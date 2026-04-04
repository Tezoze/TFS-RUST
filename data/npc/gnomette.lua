-- Gnomette (ShopBuilder)
local npc = ShopBuilder:new("Gnomette",
	{lookType = 507, lookHead = 94, lookBody = 72, lookLegs = 115, lookFeet = 115})
npc:addBuyable("teleport crystal", 18457, 150)
npc:register()
