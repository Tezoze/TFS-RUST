-- Tibia Tales Quest
-- Converted from quests.xml to Lua
-- A collection of short quests and tales across Tibia
-- Storage keys defined in data/lib/core/storages.lua under Storage.TibiaTales

local tibiaTales = GlobalEvent("TibiaTalesQuestStart")

function tibiaTales.onStartup()
	local quest = Game.createQuest("Tibia Tales", {
		storageId = Storage.TibiaTales.DefaultStart,  -- 81000
		storageValue = 1,
		missions = {
			{
				name = "To Appease the Mighty",
				storageId = Storage.TibiaTales.ToAppeaseTheMightyQuest,  -- 81020
				startValue = 1,
				endValue = 3,
				description = {
					[1] = "Kazzan sent you to talk with Ubaid and Umar to offer an appeasement treaty to the Djinn races. Talk to Umar first.",
					[2] = "Umar and Ubaid said they won't be part of those plans. Return to Kazzan and collect your reward.",
					[3] = "You have completed the quest!"
				}
			},
			{
				name = "Arito's Task",
				storageId = Storage.TibiaTales.AritosTask,  -- 81021
				startValue = 1,
				endValue = 3,
				description = {
					[1] = "Arito asked you to make a peace agreement between him and the Nomads. Go to the nomad cave north of Ankrahmun. Place a scimitar to your left and pour water to your right to reveal the entrance.",
					[2] = "Muhad has acquitted Arito. Return to Arito and tell him the good news.",
					[3] = "Arito is now safe and you have access to the Nomads Cave."
				}
			},
			{
				name = "Against the Spider Cult",
				storageId = Storage.TibiaTales.AgainstTheSpiderCult,  -- 81022
				startValue = 1,
				endValue = 6,
				description = {
					[1] = "Daniel Steelsoul in Edron wants you to infiltrate the Edron Orc Cave and destroy 4 Spider Eggs.",
					[2] = "You destroyed 1 of 4 Spider Eggs in the Edron Orc Cave.",
					[3] = "You destroyed 2 of 4 Spider Eggs in the Edron Orc Cave.",
					[4] = "You destroyed 3 of 4 Spider Eggs in the Edron Orc Cave.",
					[5] = "You destroyed all Spider Eggs in the Edron Orc Cave, report back to Daniel Steelsoul!",
					[6] = "You have completed the Quest!"
				}
			},
			{
				name = "An Interest In Botany",
				storageId = Storage.TibiaTales.AnInterestInBotany,  -- 81023
				startValue = 1,
				endValue = 4,
				description = {
					[1] = "Rabaz in Farmine asked you to collect samples from rare plant specimen in Zao. Go to the storage room to the west and receive the Botany Almanach. Find then the Giant Dreadcoil and use your Obsidian Knife on it to obtain a sample.",
					[2] = "Now you must find the second plant, a Giant Verminous and use your Obsidian Knife on it to obtain a sample.",
					[3] = "You found the two samples, report back to Rabaz in Farmine!",
					[4] = "You have completed the Quest!"
				}
			},
			{
				name = "Into the Bone Pit",
				storageId = Storage.TibiaTales.IntoTheBonePit,  -- 81025
				startValue = 1,
				endValue = 3,
				description = {
					[1] = "Search the Cursed Bone Pit in the dungeon north of Thais and dig for a well-preserved human bone for Muriel.",
					[2] = "You have found a desecrated bone for Muriel.",
					[3] = "You helped Muriel to obtain a desecrated bone."
				}
			},
			{
				name = "Rest in Hallowed Ground",
				storageId = Storage.TibiaTales.RestInHallowedGround.Questline,  -- 81002
				startValue = 1,
				endValue = 5,
				description = {
					[1] = "Go to the white raven monastery and ask for some holy water for Amanda.",
					[2] = "You got the holy water from the white raven monastery. Go back to Amanda and report about your mission.",
					[3] = "Sanctify every single grave at the unholy graveyard north of Edron with the holy water.",
					[4] = "You have sanctified all graves at the unholy graveyard of Edron. Report about your mission at Amanda.",
					[5] = "You helped Amanda by sanctifying the cursed graveyard of Edron."
				}
			},
			{
				name = "The Exterminator",
				storageId = Storage.TibiaTales.TheExterminator,  -- 81026
				startValue = 1,
				endValue = 3,
				description = {
					[1] = "Padreia in Carlin asked you to exterminate the slimes in the sewers of Carlin by poisoning their spawn pool.",
					[2] = "You poisoned the spawn pool of the slimes in the sewers of Carlin. Report to Padreia about your mission.",
					[3] = "You successfully helped Padreia in saving Carlin from a slimy disease."
				}
			},
			{
				name = "The Ultimate Booze",
				storageId = Storage.TibiaTales.ultimateBoozeQuest,  -- 81027
				startValue = 1,
				endValue = 3,
				description = {
					[1] = "Boozer in Venore asked you to bring him some special dwarven brown ale. You may find some in the brewery in Kazordoon.",
					[2] = "You found the special dwarven brown ale. Bring it to Boozer in Venore.",
					[3] = "You have completed The Ultimate Booze Quest!"
				}
			},
			{
				name = "Nomads Land",
				storageId = Storage.TibiaTales.NomadsLand,  -- 81028
				startValue = 1,
				endValue = 3,
				description = {
					[1] = "Muhad asked you to retrieve a stolen treasure from thieves in Ankrahmun. Find a pillar with a hawk symbol and press the eye to reveal a secret passage.",
					[2] = "You found the thieves' hideout. Search for a small casket containing the nomads' treasure.",
					[3] = "You have returned the treasure to Muhad and completed the Nomads Land quest."
				}
			}
		}
	})

	quest:register()
	print(">> Registered quest: Tibia Tales (9 missions)")
	return true
end

tibiaTales:register()
