-- Sam's Old Backpack Quest
-- Converted from quests.xml to Lua
-- Storage keys defined in data/lib/core/storages.lua under Storage.SamsOldBackpack

local samsOldBackpackQuest = GlobalEvent("SamsOldBackpackQuestStart")

function samsOldBackpackQuest.onStartup()
	local quest = Game.createQuest("Sam's Old Backpack", {
		storageId = Storage.SamsOldBackpack,  -- 30270
		storageValue = 1,
		missions = {
			{
				name = "Dwarven Armor Quest",
				storageId = Storage.SamsOldBackpack,  -- 30270
				startValue = 1,
				endValue = 3,
				description = {
					[1] = "Sam sends you to see Kroox in Kazordoon to get a special dwarven armor. Just tell him, his old buddy Sam is sending you.",
					[2] = "You have the permission to retrive a dwarven armor from the mines. The problem is, some giant spiders made the tunnels where the storage is their new home.",
					[3] = "You have retrieved the dwarven armor as a reward for returning Sam's old backpack to him."
				}
			}
		}
	})

	quest:register()
	print(">> Registered quest: Sam's Old Backpack (1 mission)")
	return true
end

samsOldBackpackQuest:register()
