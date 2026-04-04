-- Kihil, The Guard (DialogueBuilder)
local npc = DialogueBuilder:new("Kihil, The Guard",
	{lookType = 70})
npc:walkInterval(0)
npc:greetMessage("Greetings, citizen. How may I assist you?")
npc:farewellMessage("Stay safe out there.")
npc:walkawayMessage("Farewell.")
npc:register()
