-- Amarie (DialogueBuilder)
local npc = DialogueBuilder:new("Amarie",
	{lookType = 159, lookHead = 128, lookBody = 34, lookLegs = 28, lookFeet = 116})
npc:greetWords({"hi", "hello", "ashari"})
npc:addVoice("Please leave me alone... I have to study.")
npc:register()
