-- Dener Diegoli (DialogueBuilder)
local npc = DialogueBuilder:new("Dener Diegoli",
	{lookType = 97, lookHead = 114, lookBody = 114, lookLegs = 114, lookFeet = 114, lookAddons = 3})
npc:greetMessage("Greetings |PLAYERNAME|. Will you help me? If you do, I'll reward you with nice addons! Just say {addons} or {help} if you don't know what to do.")
npc:register()
