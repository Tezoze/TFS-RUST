-- Addoner (DialogueBuilder)
local npc = DialogueBuilder:new("Addoner",
	{lookType = 134, lookHead = 78, lookBody = 88, lookFeet = 88, lookAddons = 3})
npc:greetMessage("Greetings |PLAYERNAME|. Will you help me? If you do, I'll reward you with nice addons! Just say {addons} or {help} if you don't know what to do.")
npc:register()
