-- Lyonel (ShopBuilder)
local npc = ShopBuilder:new("Lyonel",
	{lookType = 151, lookHead = 57, lookBody = 58, lookLegs = 21, lookFeet = 114})
npc:addBuyable("bread", 2689, 4)
npc:addBuyable("cheese", 2696, 6)
npc:addBuyable("flask of rum", 5553, 150)
npc:addBuyable("ham", 2671, 8)
npc:addBuyable("meat", 2666, 5)
npc:register()
