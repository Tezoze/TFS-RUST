-- Demon Skeleton (DialogueBuilder)
local npc = DialogueBuilder:new("Demon Skeleton",
	{lookType = 37})
npc:walkInterval(0)
npc:greetMessage("*The skeleton stares at you with empty eye sockets*")
npc:farewellMessage("*rattles bones*")
npc:register()
