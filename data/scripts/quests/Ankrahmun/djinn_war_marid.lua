-- The Djinn War - Marid Faction Quest
-- Converted from quests.xml to Lua
-- Storage keys defined in data/lib/core/storages.lua under Storage.DjinnWar

local djinnWarMaridQuest = GlobalEvent("DjinnWarMaridQuestStart")

function djinnWarMaridQuest.onStartup()
	local quest = Game.createQuest("The Djinn War - Marid Faction", {
		storageId = Storage.DjinnWar.MaridFaction.Start,  -- 88002
		storageValue = 1,
		missions = {
			{
				name = "Marid Mission 1: The Dwarven Kitchen",
				storageId = Storage.DjinnWar.MaridFaction.Mission01,  -- 88011
				startValue = 1,
				endValue = 2,
				description = {
					[1] = "Bring a cookbook of the dwarven kitchen to Bo'ques.",
					[2] = "You have delivered the cookbook. Bo'ques seemed very satisfied and told you that Fa'hradin might have another mission for you."
				}
			},
			{
				name = "Marid Mission 2: The Spyreport",
				storageId = Storage.DjinnWar.MaridFaction.Mission02,  -- 88012
				startValue = 1,
				endValue = 2,
				description = {
					[1] = "Fa'hradin asked you to sneak into the Efreet fortress Mal'ouqhah and find their undercover spy. The codeword is PIEDPIPER.",
					[2] = "You have delivered the spyreport. Fa'hradin seemed impressed and told you that Gabel himself might have another mission for you."
				}
			},
			{
				name = "Rata'Mari and the Cheese",
				storageId = Storage.DjinnWar.MaridFaction.RataMari,  -- 88016
				startValue = 1,
				endValue = 2,
				description = {
					[1] = "You have promised Rata'Mari cheese. Once you deliver some to him, he will hand over his spyreport.",
					[2] = "You got Rata'Mari's spyreport. He seems to be quite happy with the cheese you brought him."
				}
			},
			{
				name = "Marid Mission 3: The Sleeping Lamp",
				storageId = Storage.DjinnWar.MaridFaction.Mission03,  -- 88013
				startValue = 1,
				endValue = 3,
				description = {
					[1] = "Gabel asked you to find Fa'hradin's sleeping lamp in the orc fortress at Ulderek's Rock. Then, sneak into Mal'ouqhah and exchange Malor's sleeping lamp with Fa'hradin's lamp.",
					[2] = "You successfully exchanged the lamps. Gabel will be happy to hear about this.",
					[3] = "The Marid deeply appreciate your help. King Gabel allowed you to trade with Haroun and Nah'bob."
				}
			}
		}
	})

	quest:register()
	print(">> Registered quest: The Djinn War - Marid Faction (4 missions)")
	return true
end

djinnWarMaridQuest:register()
