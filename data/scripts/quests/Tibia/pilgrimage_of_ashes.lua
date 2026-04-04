-- Pilgrimage of Ashes Quest
-- Converted from quests.xml to Lua
-- Quest for obtaining the 5 blessings across Tibia
-- Storage keys defined in data/lib/core/storages.lua under Storage.PilgrimageOfAshes

local pilgrimageOfAshes = GlobalEvent("PilgrimageOfAshesQuestStart")

function pilgrimageOfAshes.onStartup()
	local quest = Game.createQuest("Pilgrimage of Ashes", {
		storageId = Storage.PilgrimageOfAshes.Questline,  -- 45300
		storageValue = 1,
		missions = {
			{
				name = "Spiritual Shielding",
				storageId = Storage.PilgrimageOfAshes.Mission01,  -- 45301
				startValue = 1,
				endValue = 2,
				description = {
					[1] = "Visit Norf at the Whiteflower Temple south of Thais to receive the Spiritual Shielding blessing.",
					[2] = "You have obtained the Spiritual Shielding blessing from Norf."
				}
			},
			{
				name = "Embrace of Tibia",
				storageId = Storage.PilgrimageOfAshes.Mission02,  -- 45302
				startValue = 1,
				endValue = 2,
				description = {
					[1] = "Visit Humphrey north of Carlin to receive the Embrace of Tibia blessing.",
					[2] = "You have obtained the Embrace of Tibia blessing from Humphrey."
				}
			},
			{
				name = "Fire of the Suns",
				storageId = Storage.PilgrimageOfAshes.Mission03,  -- 45303
				startValue = 1,
				endValue = 2,
				description = {
					[1] = "Visit Edala in the Suntower near Ab'Dendriel to receive the Fire of the Suns blessing.",
					[2] = "You have obtained the Fire of the Suns blessing from Edala."
				}
			},
			{
				name = "Spark of the Phoenix",
				storageId = Storage.PilgrimageOfAshes.Mission04,  -- 45304
				startValue = 1,
				endValue = 3,
				description = {
					[1] = "Visit Kawill in the earth temple to receive the first part of the Spark of the Phoenix blessing.",
					[2] = "Visit Pydar in the fire temple to receive the second part of the Spark of the Phoenix blessing.",
					[3] = "You have obtained the complete Spark of the Phoenix blessing from Kawill and Pydar."
				}
			},
			{
				name = "Wisdom of Solitude",
				storageId = Storage.PilgrimageOfAshes.Mission05,  -- 45305
				startValue = 1,
				endValue = 2,
				description = {
					[1] = "Visit Eremo on the Isle of Cormaya to receive the Wisdom of Solitude blessing.",
					[2] = "You have obtained the Wisdom of Solitude blessing from Eremo."
				}
			},
			{
				name = "Reward Claimed",
				storageId = Storage.PilgrimageOfAshes.RewardClaimed,  -- 45306
				startValue = 1,
				endValue = 1,
				description = {
					[1] = "You have claimed your reward from a city guide."
				}
			}
		}
	})

	quest:register()
	print(">> Registered quest: Pilgrimage of Ashes (6 missions)")
	return true
end

pilgrimageOfAshes:register()
