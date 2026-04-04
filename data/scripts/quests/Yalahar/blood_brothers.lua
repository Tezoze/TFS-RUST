-- Blood Brothers Quest
-- Converted from quests.xml to Lua
-- Storage keys defined in data/lib/core/storages.lua under Storage.BloodBrothers

local bloodBrothersQuest = GlobalEvent("BloodBrothersQuestStart")

function bloodBrothersQuest.onStartup()
	local quest = Game.createQuest("Blood Brothers Quest", {
		storageId = Storage.BloodBrothers.Questline,  -- 45230
		storageValue = 1,
		missions = {
			{
				name = "Mission 01: Gaining Trust",
				storageId = Storage.BloodBrothers.Mission01,  -- 45231
				startValue = 1,
				endValue = 2,
				description = {
					[1] = "Julius wants you to prove you're not a vampire. Mix flour with holy water, then use that dough on bulb of garlic to create garlic dough. Bake it and eat it in front of Julius!",
					[2] = "You have proven yourself to Julius. Return to him for your next mission."
				}
			},
			{
				name = "Mission 02: Bad Eggs",
				storageId = Storage.BloodBrothers.Mission02,  -- 45232
				startValue = 1,
				endValue = 3,
				description = {
					[1] = "Bake garlic cookies and offer them to these citizens of Yalahar: Serafin, Lisander, Ortheus, Maris, and Armenius. Watch their reactions carefully!",
					[2] = "You have identified all vampire suspects. Return to Julius to complete this task.",
					[3] = "Armenius has been identified as the most suspicious vampire. Return to Julius for your next mission."
				}
			},
			{
				name = "Mission 03: His True Face",
				storageId = Storage.BloodBrothers.Mission03,  -- 45233
				startValue = 1,
				endValue = 3,
				description = {
					[1] = "Cast the spell 'alori mort' on Armenius to reveal his true vampire nature. Face him directly when casting the spell!",
					[2] = "Armenius has been revealed as a vampire! Return to Julius to complete this task.",
					[3] = "Julius is shocked by the revelation. Return to him for your next mission."
				}
			},
			{
				name = "Mission 04: The Dark Lands",
				storageId = Storage.BloodBrothers.Mission04,  -- 45234
				startValue = 1,
				endValue = 3,
				description = {
					[1] = "Find Harlow at the Yalahar docks and travel to Vengoth. Search Vengoth for strange spots and use Julius' Map to mark them. Find a way into the vampire castle!",
					[2] = "You have entered the vampire castle! Return to Julius to complete this task.",
					[3] = "Julius is concerned about the castle's defenses. Return to him for your next mission."
				}
			},
			{
				name = "Mission 05: Into the Castle",
				storageId = Storage.BloodBrothers.Mission05,  -- 45235
				startValue = 1,
				endValue = 5,
				description = {
					[1] = "Get a Blood Crystal from the Research Centre in the Magician Quarter. Ask around Yalahar about 'blood crystal' for hints on where to find it.",
					[2] = "Go to A Wandering Soul on Vengoth and say 'blood crystal' to charge it.",
					[3] = "Your crystal is charged! Now gather 3 other players with charged blood crystals. Stand on the 4 strange carvings in Vengoth to perform the ritual.",
					[4] = "The ritual worked! Return to Julius to report your mission.",
					[5] = "The ritual worked! You have been teleported deeper into the castle. Your next mission is to find information on the castle."
				}
			},
			{
				name = "Mission 06: A Black History",
				storageId = Storage.BloodBrothers.Mission06,  -- 45236
				startValue = 1,
				endValue = 3,
				description = {
					[1] = "Enter the main hall and use the northern part of a small Old Carpet to find a Closed Trapdoor. Go downstairs and explore the castle rooms to find Arthei's diary parts.",
					[2] = "You found part of Arthei's diary! Take it back to Julius. He will reward you and may ask for more diary pages (optional for Castlemania achievement).",
					[3] = "Julius understands the threat now. The four vampire brothers must be destroyed. Return to him for your next mission."
				}
			},
			{
				name = "Mission 07: Boreth",
				storageId = Storage.BloodBrothers.Mission07,  -- 45237
				startValue = 1,
				endValue = 3,
				description = {
					[1] = "Find and destroy Boreth, the plant-obsessed vampire brother in the castle.",
					[2] = "You have destroyed Boreth! Return to Julius to complete this task.",
					[3] = "Julius is impressed by your success. Return to him for your next mission."
				}
			},
			{
				name = "Mission 08: Lersatio",
				storageId = Storage.BloodBrothers.Mission08,  -- 45238
				startValue = 1,
				endValue = 3,
				description = {
					[1] = "Find and destroy Lersatio, the vain vampire brother who longs to see his reflection again.",
					[2] = "You have destroyed Lersatio! Return to Julius to complete this task.",
					[3] = "Julius senses the vampire curse weakening. Return to him for your next mission."
				}
			},
			{
				name = "Mission 09: Marziel",
				storageId = Storage.BloodBrothers.Mission09,  -- 45239
				startValue = 1,
				endValue = 3,
				description = {
					[1] = "Find and destroy Marziel, the tormented author of the diary. Note: This mission typically requires a female character.",
					[2] = "You have destroyed Marziel! Return to Julius to complete this task.",
					[3] = "Julius feels the end is near. Return to him for the final mission."
				}
			},
			{
				name = "Mission 10: Arthei",
				storageId = Storage.BloodBrothers.Mission10,  -- 45240
				startValue = 1,
				endValue = 3,
				description = {
					[1] = "Face Arthei, the master of the vampire brothers, and end this curse once and for all!",
					[2] = "You have destroyed Arthei! Return to Julius to complete this task.",
					[3] = "You have completed the Blood Brothers Quest! Julius offers you a choice of rewards: vampiric crest or yalaharian outfit addon."
				}
			}
		}
	})

	quest:register()
	print(">> Registered quest: Blood Brothers Quest (10 missions)")
	return true
end

bloodBrothersQuest:register()
