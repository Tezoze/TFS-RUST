-- Pits of Inferno Quest
-- Converted from quests.xml to Lua
-- Location: Plains of Havoc
-- Storage keys defined in data/lib/core/storages.lua under Storage.PitsOfInferno

local pitsOfInferno = GlobalEvent("PitsOfInfernoQuest")

function pitsOfInferno.onStartup()
	local quest = Game.createQuest("Pits of Inferno", {
		storageId = Storage.PitsOfInferno.ThroneInfernatil,  -- 30035
		storageValue = 1,
		missions = {
			{
				name = "Infernatil's Throne",
				storageId = Storage.PitsOfInferno.ThroneInfernatil,  -- 30035
				startValue = 1,
				endValue = 1,
				description = {
					[1] = "You have touched Infernatil's throne and absorbed some of his spirit."
				}
			},
			{
				name = "Tafariel's Throne",
				storageId = Storage.PitsOfInferno.ThroneTafariel,  -- 30036
				startValue = 1,
				endValue = 1,
				description = {
					[1] = "You have touched Tafariel's throne and absorbed some of his spirit."
				}
			},
			{
				name = "Vermior's Throne",
				storageId = Storage.PitsOfInferno.ThroneVerminor,  -- 30037
				startValue = 1,
				endValue = 1,
				description = {
					[1] = "You have touched Vermior's throne and absorbed some of his spirit."
				}
			},
			{
				name = "Apocalypse's Throne",
				storageId = Storage.PitsOfInferno.ThroneApocalypse,  -- 30038
				startValue = 1,
				endValue = 1,
				description = {
					[1] = "You have touched Apocalypse's throne and absorbed some of his spirit."
				}
			},
			{
				name = "Bazir's Throne",
				storageId = Storage.PitsOfInferno.ThroneBazir,  -- 30039
				startValue = 1,
				endValue = 1,
				description = {
					[1] = "You have touched Bazir's throne and absorbed some of his spirit."
				}
			},
			{
				name = "Ashfalor's Throne",
				storageId = Storage.PitsOfInferno.ThroneAshfalor,  -- 30040
				startValue = 1,
				endValue = 1,
				description = {
					[1] = "You have touched Ashfalor's throne and absorbed some of his spirit."
				}
			},
			{
				name = "Pumin's Throne",
				storageId = Storage.PitsOfInferno.ThronePumin,  -- 30041
				startValue = 1,
				endValue = 1,
				description = {
					[1] = "You have touched Pumin's throne and absorbed some of his spirit."
				}
			},
			{
				name = "Rewards",
				storageId = Storage.PitsOfInferno.WeaponReward,  -- 30042
				startValue = 1,
				endValue = 1,
				description = {
					[1] = "You have claimed your rewards from the Pits of Inferno."
				}
			},
			{
				name = "Shortcut Access",
				storageId = Storage.PitsOfInferno.ShortcutHub,  -- 30043
				startValue = 1,
				endValue = 1,
				description = {
					[1] = "You have unlocked the shortcut in the Pits of Inferno."
				}
			}
		}
	})

	quest:register()
	print(">> Registered quest: Pits of Inferno (9 missions)")
	return true
end

pitsOfInferno:register()
