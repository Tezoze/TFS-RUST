-- Test Dialogue NPC using the new NpcBuilder framework
-- This validates DialogueBuilder with keyword responses, custom greet words, and voice.

local npc = DialogueBuilder:new("Test King", {lookType = 128, lookHead = 95, lookBody = 116, lookLegs = 114, lookFeet = 0})
npc:greetWords({"hail king", "hello king", "hi king"})  -- custom greet words
npc:greetMessage("Greetings, |PLAYERNAME|. What brings you before the king?")
npc:farewellMessage("You may leave now, |PLAYERNAME|.")
npc:addResponse({"job", "work"}, "I am the king of this test realm.")
npc:addResponse("name", "I am the Test King, ruler of all test NPCs.")
npc:addResponse({"quest", "mission"}, "I have no quests for you at this time.")
npc:addResponse("army", "My army is strong and well-trained.")
npc:addVoice("Long live the king!")
npc:voiceInterval(30)
npc:voiceChance(20)
npc:register()
