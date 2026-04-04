-- Factions Quest (Djinn War)
-- Converted from quests.xml to Lua
-- Storage keys defined in data/lib/core/storages.lua under Storage.DjinnWar.Faction

local factionsQuest = GlobalEvent("FactionsQuestStart")

function factionsQuest.onStartup()
	local quest = Game.createQuest("Factions", {
		storageId = Storage.DjinnWar.Faction.Greeting,  -- 50723
		storageValue = 2,
		missions = {
			{
				name = "The Marid and the Efreet - Djinn Greeting",
				storageId = Storage.DjinnWar.Faction.Greeting,  -- 50723
				startValue = 1,
				endValue = 2,
				description = {
					[1] = "Melchior told you the word \"Djanni'hah\" which can be used to talk to Djinns. Be aware that once you become an ally of one Djinn race, you cannot switch sides anymore.",
					[2] = "You have learned the Djinn greeting and can now communicate with the Djinn races."
				}
			},
			{
				name = "The Marid and the Efreet - Marid Faction",
				storageId = Storage.DjinnWar.Faction.Marid,  -- 30034
				startValue = 1,
				endValue = 2,
				description = {
					[1] = "You have joined the Marid. These friendly, blue Djinns are honest and fair allies. You have pledged eternal loyalty to King Gabel and may enter Asha'daramai freely. Djanni'hah!",
					[2] = "You are now a full member of the Marid faction with access to their territories and services."
				}
			},
			{
				name = "The Efreet and the Efreet - Efreet Faction",
				storageId = Storage.DjinnWar.Faction.Efreet,  -- 30033
				startValue = 1,
				endValue = 2,
				description = {
					[1] = "You have joined the Efreet. These evil, green Djinns are always up to mischievous pranks. You have pledged eternal loyalty to King Malor and may enter Mal'ouquah freely. Djanni'hah!",
					[2] = "You are now a full member of the Efreet faction with access to their territories and services."
				}
			}
		}
	})

	quest:register()
	print(">> Registered quest: Factions (3 missions)")
	return true
end

factionsQuest:register()
