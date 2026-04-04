-- Jack (DialogueBuilder)
local npc = DialogueBuilder:new("Jack",
	{lookType = 128, lookHead = 115, lookBody = 96, lookLegs = 115, lookFeet = 114, lookAddons = 3})
npc:greetMessage("Yes? What can I do for you? I hope this won't take long, though.")
npc:register()
