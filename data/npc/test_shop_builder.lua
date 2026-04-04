-- Test Shop NPC using the new NpcBuilder framework
-- This validates ShopBuilder with custom greet words and buy/sell items.

local npc = ShopBuilder:new("Test Merchant", {lookType = 131, lookHead = 38, lookBody = 113, lookLegs = 67, lookFeet = 95})
npc:greetMessage("Welcome to my test shop, |PLAYERNAME|! Ask me for a {trade}.")
npc:farewellMessage("Good bye, |PLAYERNAME|. Come again!")
npc:greetWords({"hi", "hello", "hey"})  -- custom greet words
npc:addKeyword("job", "I am a test merchant for the new NPC builder framework.")
npc:addKeyword("name", "I am the Test Merchant.")
npc:addVoice("Testing, testing... is this thing on?")
npc:addBuyableAndSellable("sword", 2376, 85, 25)
npc:addBuyableAndSellable("plate armor", 2463, 1200, 400)
npc:addBuyable("magic plate armor", 2472, 90000)
npc:addSellable("magic plate armor", 2472, 67000)
npc:register()
