-- An Uneasy Alliance Quest
-- Converted from quests.xml to Lua
-- Storage keys defined in data/lib/core/storages.lua under Storage.AnUneasyAlliance

local anUneasyAlliance = GlobalEvent("AnUneasyAllianceQuestStart")

function anUneasyAlliance.onStartup()
	local quest = Game.createQuest("An Uneasy Alliance Quest", {
		storageId = Storage.AnUneasyAlliance,  -- 45220
		storageValue = 1,
		endValue = 5,
		missions = {
			{
				name = "The Wrath of the Kahn",
				storageId = Storage.AnUneasyAlliance,  -- 45220
				startValue = 1,
				endValue = 3,
				description = {
					[1] = "Mission: kill Renegade Orc!",
					[2] = "Return to Curos and report your mission.",
					[3] = "You have completed this mission!"
				}
			},
			{
				name = "The Maw of the Dragon",
				storageId = Storage.AnUneasyAlliance,  -- 45220
				startValue = 3,
				endValue = 5,
				description = {
					[3] = "Destroy the magical scrying device in the lizard tower.",
					[4] = "Return to Curos and report your mission.",
					[5] = "You have destroyed the scrying device!"
				}
			},
			{
				name = "Milk Delivery",
				storageId = Storage.AnUneasyAllianceTasks.MilkDelivery,  -- 45221
				startValue = 1,
				endValue = 2,
				description = {
					[1] = "Bring a bucket of milk to Curos.",
					[2] = "You have completed the milk delivery task."
				}
			},
			{
				name = "To Feed a Beast",
				storageId = Storage.AnUneasyAllianceTasks.FeedBeast,  -- 45222
				startValue = 1,
				endValue = 2,
				description = {
					[1] = "Bring a caiman steak to Curos.",
					[2] = "You have completed the beast feeding task."
				}
			},
			{
				name = "Honour The Dead",
				storageId = Storage.AnUneasyAllianceTasks.HonourDead,  -- 45223
				startValue = 1,
				endValue = 2,
				description = {
					[1] = "Meditate on the orc grave in the mountains north of Zao.",
					[2] = "You have honoured the dead."
				}
			},
			{
				name = "Foul Spirits",
				storageId = Storage.AnUneasyAllianceTasks.FoulSpirits,  -- 45224
				startValue = 1,
				endValue = 2,
				description = {
					[1] = "Play a flute to appease the spirits near the haunted trees.",
					[2] = "You have appeased the foul spirits."
				}
			}
		}
	})

	quest:register()
	print(">> Registered quest: An Uneasy Alliance Quest (6 missions)")
	return true
end

anUneasyAlliance:register()
