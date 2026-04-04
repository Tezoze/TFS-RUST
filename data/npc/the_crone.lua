-- The Crone (DialogueBuilder)
local npc = DialogueBuilder:new("The Crone",
	{lookType = 78})
npc:greetMessage("Be greeted, |PLAYERNAME|... mortal.")
npc:addVoice("Let me mourn in peace.")
npc:register()
