-- Rotem Valos (DialogueBuilder)
local npc = DialogueBuilder:new("Rotem Valos",
	{lookType = 335, lookHead = 79, lookBody = 77, lookLegs = 79, lookFeet = 94, lookAddons = 2})
npc:walkInterval(0)
npc:addVoice("<sigh> The world has grown complicated since my youth.")
npc:register()
