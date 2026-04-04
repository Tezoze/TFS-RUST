-- Test Travel NPC using the new NpcBuilder framework
-- This validates TravelBuilder with destinations, premium check, and PZ lock.

local npc = TravelBuilder:new("Test Captain", {lookType = 129, lookHead = 19, lookBody = 69, lookLegs = 125, lookFeet = 50})
npc:greetMessage("Welcome on board, |PLAYERNAME|. Where do you want to go?")
npc:farewellMessage("Good bye, |PLAYERNAME|. Safe travels!")
npc:addDestination("Thais", {x = 32310, y = 32210, z = 6}, 170, false)
npc:addDestination("Carlin", {x = 32387, y = 31820, z = 6}, 110, false)
npc:addDestination("Edron", {x = 33175, y = 31764, z = 6}, 160, true) -- premium only
npc:addKeyword("job", "I am a test captain for the new NPC builder framework.")
npc:addVoice("Passages to Thais, Carlin, and Edron!")
npc:register()
