-- Golem Guardian (DialogueBuilder)
local npc = DialogueBuilder:new("Golem Guardian",
	{lookType = 326})
npc:walkInterval(0)
npc:greetMessage("*The golem's eyes glow as it acknowledges your presence*")
npc:farewellMessage("*The golem returns to its dormant state*")
npc:register()
