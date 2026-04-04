-- Captain Cookie (DialogueBuilder)
local npc = DialogueBuilder:new("Captain Cookie",
	{lookType = 96})
npc:greetMessage("Greetings, daring adventurer. If you need a {passage}, let me know.")
npc:farewellMessage("Good bye.")
npc:walkawayMessage("Good bye.")
npc:register()
