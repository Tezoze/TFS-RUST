-- Mugluf (ShopBuilder)
local npc = ShopBuilder:new("Mugluf",
	{lookType = 146, lookHead = 95, lookFeet = 113})
npc:walkInterval(0)
npc:addBuyable("ham", 2671, 8)
npc:addBuyable("meat", 2666, 5)
npc:addBuyable("salmon", 2668, 6)
npc:register()
