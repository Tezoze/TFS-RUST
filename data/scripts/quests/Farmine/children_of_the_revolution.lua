-- Children of the Revolution Quest
-- Converted from quests.xml to Lua
-- Storage keys defined in data/lib/core/storages.lua under Storage.ChildrenoftheRevolution

local childrenRevolution = GlobalEvent("ChildrenOfTheRevolutionQuestStart")

function childrenRevolution.onStartup()
	local quest = Game.createQuest("Children of the Revolution", {
		storageId = Storage.ChildrenoftheRevolution.Questline,
		storageValue = 1,
		missions = {
			{
				name = "Prove Your Worzz!",
				storageId = Storage.ChildrenoftheRevolution.Mission00,
				startValue = 1,
				endValue = 2,
				description = {
					[1] = "Your Mission is to go to a little camp of lizards at north-east of the Dragonblaze Peaks. You have to find and deliver the Tactical map complete the mission.",
					[2] = "You delivered the Tactical map to Zalamon."
				}
			},
			{
				name = "Mission 1: Corruption",
				storageId = Storage.ChildrenoftheRevolution.Mission01,
				startValue = 1,
				endValue = 3,
				description = {
					[1] = "Go to the Temple of Equilibrium (it's marked on your map) and find out what happened there.",
					[2] = "The temple has been corrupted and is lost. Zalamon should be informed about this as soon as possible.",
					[3] = "You already reported Zalamon about the Temple! Ask him for new mission!"
				}
			},
			{
				name = "Mission 2: Imperial Zzecret Weaponzz",
				storageId = Storage.ChildrenoftheRevolution.Mission02,
				startValue = 1,
				endValue = 5,
				description = {
					[1] = "Go into the small camp Chaochai to the north of the Dragonblaze Peaks (Zalamon marks the entrance on your map). There are 3 buildings which you have to spy",
					[2] = "You spied 1 of 3 buildings of the camp.",
					[3] = "You spied 2 of 3 buildings of the camp.",
					[4] = "You spied 3 of 3 buildings of the camp. Zalamon should be informed about this as soon as possible.",
					[5] = "You already reported Zalamon about the camp! Ask him for new mission!"
				}
			},
			{
				name = "Mission 3: Zee Killing Fieldzz",
				storageId = Storage.ChildrenoftheRevolution.Mission03,
				startValue = 1,
				endValue = 3,
				description = {
					[1] = "Get the poison from Zalamon's storage room. Then go to the teleporter to the Muggy Plains and head east from there to the rice fields. Go to the very top rice field and use the poison anywhere on the water.",
					[2] = "The rice has been poisoned. This will weaken the Emperor's army significantly. Return and tell Zalamon about your success.",
					[3] = "You already reported Zalamon about your success! Ask him for new mission!"
				}
			},
			{
				name = "Mission 4: Zze Way of Zztonezz",
				storageId = Storage.ChildrenoftheRevolution.Mission04,
				startValue = 1,
				endValue = 6,
				description = {
					[1] = "Your mission is to find a way to enter the north of the valley and find a passage to the great gate itself. Search any temples or settlements you come across for hidden passages.",
					[2] = "Report Zalamon about the strange symbols that you found.",
					[3] = "Get the greasy oil from Zalamon's storage room and put them on the levers that you found.",
					[4] = "Due to being extra greasy, the leavers can now be moved.",
					[5] = "You found the right combination for the puzzle in the mountains and triggered some kind of mechanism. You should head back to Zalamon to report your success.",
					[6] = "You already reported Zalamon about your success! You got a Tome of Knowledge as reward! Ask him for new mission!"
				}
			},
			{
				name = "Mission 5: Phantom Army",
				storageId = Storage.ChildrenoftheRevolution.Mission05,
				startValue = 1,
				endValue = 3,
				description = {
					[1] = "Your mission is to use the portal in the chamber beyond the mechanism. It will lead you to the great gate.",
					[2] = "Eternal guardians and lizard chosen has been awaken. Survive them and report it to Zalamon!",
					[3] = "You Survived the Waves and reported Zalamon about your success! You got a Serpent Crest as reward!"
				}
			}
		}
	})

	quest:register()
	print(">> Registered quest: Children of the Revolution (6 missions)")
	return true
end

childrenRevolution:register()
