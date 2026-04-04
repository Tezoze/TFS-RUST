-- Boveas (DialogueBuilder)
local npc = DialogueBuilder:new("Boveas",
	{lookType = 25})
npc:greetMessage("Hi! I hope you're not going to kill me!")
npc:farewellMessage("Good bye, |PLAYERNAME|.")
npc:walkawayMessage("Good bye, |PLAYERNAME|.")
npc:register()
