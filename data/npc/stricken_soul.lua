-- Stricken Soul (DialogueBuilder)
local npc = DialogueBuilder:new("Stricken Soul",
	{lookType = 48})
npc:greetMessage("Greetings, |PLAYERNAME|.")
npc:addResponse("job", "Esperando Script.")
npc:register()
