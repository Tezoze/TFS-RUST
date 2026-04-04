-- Frok, The Guard (DialogueBuilder)
local npc = DialogueBuilder:new("Frok, The Guard",
	{lookType = 70})
npc:walkInterval(0)
npc:greetMessage("Halt! State your business, citizen.")
npc:farewellMessage("Move along.")
npc:walkawayMessage("Hmph.")
npc:register()
