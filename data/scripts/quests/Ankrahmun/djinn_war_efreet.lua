-- The Djinn War - Efreet Faction Quest
-- Converted from quests.xml to Lua
-- Storage keys defined in data/lib/core/storages.lua under Storage.DjinnWar

local djinnWarEfreetQuest = GlobalEvent("DjinnWarEfreetQuestStart")

function djinnWarEfreetQuest.onStartup()
	local quest = Game.createQuest("The Djinn War - Efreet Faction", {
		storageId = Storage.DjinnWar.EfreetFaction.Start,  -- 88001
		storageValue = 1,
		missions = {
			{
				name = "Efreet Mission 1: The Supply Thief",
				storageId = Storage.DjinnWar.EfreetFaction.Mission01,  -- 88006
				startValue = 1,
				endValue = 3,
				description = {
					[1] = "Travel to Thais and keep your eyes open for something that might give you a clue on the supply thief.",
					[2] = "You have found the potential supply thief - Partos in Thais seemed very suspicious. Baa'leal might be interested in this matter.",
					[3] = "You have reported the case to Baa'leal. He seemed very satisfied and told you that Alesar might have another mission for you."
				}
			},
			{
				name = "Efreet Mission 2: The Tear of Daraman",
				storageId = Storage.DjinnWar.EfreetFaction.Mission02,  -- 88007
				startValue = 1,
				endValue = 3,
				description = {
					[1] = "Sneak into Ashta'daramai and steal a \"Tear of Daraman\". For more information about these gems visit the Efreet library.",
					[2] = "You have successfully managed to steal a Tear of Daraman from Ashta'daramai. Bring it to Alesar.",
					[3] = "You have delivered Daraman's Tear. Alesar seemed very satisfied and told you that Malor himself might have another mission for you."
				}
			},
			{
				name = "Efreet Mission 3: The Sleeping Lamp",
				storageId = Storage.DjinnWar.EfreetFaction.Mission03,  -- 88008
				startValue = 1,
				endValue = 3,
				description = {
					[1] = "Malor asked you to find Fa'hradin's sleeping lamp in the orc fortress at Ulderek's Rock. Then, sneak into Ashta'daramai and exchange Gabel's sleeping lamp with Fa'hradin's lamp.",
					[2] = "You successfully exchanged the lamps. Malor will be happy to hear about this.",
					[3] = "The Efreet are very satisfied with your help. King Malor allowed you to trade with Yaman and Alesar."
				}
			}
		}
	})

	quest:register()
	print(">> Registered quest: The Djinn War - Efreet Faction (3 missions)")
	return true
end

djinnWarEfreetQuest:register()
