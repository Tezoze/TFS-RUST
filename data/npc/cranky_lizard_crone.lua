-- Cranky Lizard Crone (DialogueBuilder)
local npc = DialogueBuilder:new("Cranky Lizard Crone",
	{lookType = 339})
npc:addResponse("addons", "I can offer you first & second addons of the following outfit: {Wayfarer}.")
npc:addResponse("wayfarer", "Ask about {first addon} or {second addon}.")
npc:addResponse("outfit", "Ask about {first addon} or {second addon}.")
npc:addResponse("mission", "Ask about {first addon} or {second addon}.")
npc:addResponse("help", "llected all the required pieces, say 'yes' and voila - you got yourself an addon!")
npc:register()
