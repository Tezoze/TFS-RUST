-- Scott (ShopBuilder)
local npc = ShopBuilder:new("Scott",
	{lookType = 131, lookHead = 75, lookBody = 38, lookLegs = 77, lookFeet = 116})
npc:addBuyable("bread", 2689, 4)
npc:addBuyable("cheese", 2696, 6)
npc:addBuyable("ham", 2671, 8)
npc:addBuyable("meat", 2666, 5)
npc:addVoice("Hey there, adventurer! Need a little rest in my inn? Have some food!")
npc:register()
