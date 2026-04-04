-- Edowir (DialogueBuilder)
local npc = DialogueBuilder:new("Edowir",
	{lookType = 130, lookBody = 58, lookLegs = 96, lookFeet = 95})
npc:greetMessage("Oh, hello |PLAYERNAME|! How nice of you to visit an old man like me.")
npc:farewellMessage("Come back whenever you're in need of wisdom.")
npc:walkawayMessage("Come back whenever you're in need of wisdom.")
npc:addResponse("job", "I gather wisdom and knowledge. I am also an astrologer.")
npc:addVoice("I'm just an old man, but I know a lot about Tibia.")
npc:register()
