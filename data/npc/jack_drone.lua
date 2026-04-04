-- Jack Drone (DialogueBuilder)
local npc = DialogueBuilder:new("Jack Drone",
	{lookType = 93, lookHead = 19, lookBody = 69, lookLegs = 125, lookFeet = 50})
npc:greetMessage("Welcome on board, |PLAYERNAME|. Where can I {sail} you today? I can travel you back to {thais}.")
npc:farewellMessage("Good bye. Recommend us if you were satisfied with our service.")
npc:walkawayMessage("Good bye then.")
npc:register()
