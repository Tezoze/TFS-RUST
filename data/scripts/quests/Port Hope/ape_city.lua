-- The Ape City Quest
-- Converted from quests.xml to Lua
-- Quest NPC: Hairycles in Banuta
-- Storage key: Questline = 30285

local apeCityQuest = GlobalEvent("ApeCityQuestStart")

function apeCityQuest.onStartup()
	local quest = Game.createQuest("The Ape City", {
		storageId = Storage.TheApeCity.Questline,  -- 30285
		storageValue = 1,
		missions = {
			{
				name = "Hairycles' Missions",
				storageId = Storage.TheApeCity.Questline,  -- 30285
				startValue = 1,
				endValue = 18,
				ignoreEndValue = true,  -- Show mission as long as value >= startValue
				description = {
					[1] = "Mission 1: Find whisper moss in the dworc settlement south of Port Hope and bring it back to Hairycles.",
					[2] = "Mission 1 Complete: Hairycles was happy about the whisper moss. He might have another mission for you.",
					[3] = "Mission 2: Hairycles asked you to bring him cough syrup from a human settlement. A healer might know more about this medicine.",
					[4] = "Mission 2 Complete: Hairycles was happy about the cough syrup. He might have another mission for you.",
					[5] = "Mission 3: Hairycles asked you to bring him a magical scroll from the lizard settlement Chor.",
					[6] = "Mission 3 Complete: Hairycles appreciated the scroll and will try to read it. Maybe he has another mission for you later.",
					[7] = "Mission 4: Since Hairycles was not able to read the scroll, he asked you to dig for a tomb in the desert to the east. Find an obelisk between red stones and read it.",
					[8] = "Mission 4 Complete: Hairycles read your mind and can now translate the lizard scroll. He might have another mission for you.",
					[9] = "Mission 5: Hairycles wants to create a life charm for the ape people. He needs a hydra egg since it has strong regenerating powers.",
					[10] = "Mission 5 Complete: Hairycles attempts to create a might charm for the protection of the ape people. He might have another mission for you later.",
					[11] = "Mission 6: Hairycles needs a witches' cap mushroom which is supposed to be hidden in a dungeon deep under Fibula.",
					[12] = "Mission 6 Complete: You brought the witches' cap mushroom back to Hairycles. He might have another mission for you.",
					[13] = "Mission 7: Hairycles is worried about an ape cult which drinks some strange fluid. Go to the old lizard temple under Banuta and destroy three of the casks there with a crowbar.",
					[14] = "Mission 7 Complete: You destroyed three of the casks with snake blood. Hairycles might have another mission for you.",
					[15] = "Mission 8: The apes need a symbol of their faith. Speak with the blind prophet in a cave to the northeast and go to the Forbidden Land. Find a hair of the giant, holy ape Bong and bring it back.",
					[16] = "Mission 8 Complete: Hairycles gladly accepted the hair of the ape god. He told you to have one final mission for you.",
					[17] = "Mission 9: Go into the deepest catacombs under Banuta and destroy the monument of the snake god with the hammer that Hairycles gave to you.",
					[18] = "Quest Complete: You successfully destroyed the monument of the snake god. You can buy sacred statues from Hairycles and ask him for a shaman outfit."
				}
			}
		}
	})

	quest:register()
	print(">> Registered quest: The Ape City (1 mission, 18 states)")
	return true
end

apeCityQuest:register()
