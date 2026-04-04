-- Telas Golem (DialogueBuilder)
local npc = DialogueBuilder:new("Telas Golem",
	{lookType = 304})
npc:greetMessage("Where .. am I?")
npc:farewellMessage("Good .. bye.")
npc:walkawayMessage("Good .. bye.")
npc:addVoice("What .. happened?")
npc:register()
