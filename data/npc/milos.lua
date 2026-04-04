-- Milos (DialogueBuilder)
local npc = DialogueBuilder:new("Milos",
	{lookType = 130, lookHead = 19, lookBody = 3, lookLegs = 3, lookFeet = 2})
npc:greetMessage("Oh hello. I hardly noticed you. I'm afraid I am a bit distracted at the moment.")
npc:addVoice("What a fascinating idea!")
npc:register()
