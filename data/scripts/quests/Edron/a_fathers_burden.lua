-- A Father's Burden Quest
-- Converted from quests.xml to Lua
-- Storage keys defined in data/lib/core/storages.lua under Storage.FathersBurdenQuest

local fathersBurden = GlobalEvent("FathersBurdenQuestStart")

function fathersBurden.onStartup()
	local quest = Game.createQuest("A Father's Burden", {
		storageId = Storage.FathersBurdenQuest.QuestLog,  -- 50203
		storageValue = 1,
		missions = {
			{
				name = "The Birthday Presents",
				storageId = Storage.FathersBurdenQuest.Status,  -- 50205
				startValue = 1,
				endValue = 2,
				description = {
					[1] = "Gather the material Tereban listed. Talk to him about your mission when you have given him everything he was looking for.",
					[2] = "You brought all the required materials to Tereban and guaranteed his sons a great birthday party."
				}
			},
			{
				name = "The Magic Bow - Sinew",
				storageId = Storage.FathersBurdenQuest.Sinew,  -- 50206
				startValue = 1,
				endValue = 2,
				description = {
					[1] = "Find the wyvern Heoni in the Edron mountains and take his sinew to Tereban.",
					[2] = "You delivered Heoni's sinew to Tereban."
				}
			},
			{
				name = "The Magic Bow - Wood",
				storageId = Storage.FathersBurdenQuest.Wood,  -- 50207
				startValue = 1,
				endValue = 2,
				description = {
					[1] = "Find the special wood in the barbarian camps of Hrodmir and bring it to Tereban. It might be a good idea to start looking in the northernmost camp.",
					[2] = "You delivered the Wood to Tereban."
				}
			},
			{
				name = "The Magic Robe - Cloth",
				storageId = Storage.FathersBurdenQuest.Cloth,  -- 50208
				startValue = 1,
				endValue = 2,
				description = {
					[1] = "Find the spectral cloth hidden deep in the crypts of the isle of the kings and bring it to Tereban. You might have to look for a secret entrance.",
					[2] = "You delivered the spectral cloth to Tereban."
				}
			},
			{
				name = "The Magic Robe - Silk",
				storageId = Storage.FathersBurdenQuest.Silk,  -- 50209
				startValue = 1,
				endValue = 2,
				description = {
					[1] = "Find exquisite silk in the spider caves of southern Zao and deliver it to Tereban.",
					[2] = "You brought Tereban the required silk."
				}
			},
			{
				name = "The Magic Rod - Crystal",
				storageId = Storage.FathersBurdenQuest.Crystal,  -- 50210
				startValue = 1,
				endValue = 2,
				description = {
					[1] = "Find a magic crystal in the tomb buried under the sand east of Ankrahmun and bring it to Tereban.",
					[2] = "Tereban received the magic crystal he was looking for."
				}
			},
			{
				name = "The Magic Rod - Root",
				storageId = Storage.FathersBurdenQuest.Root,  -- 50211
				startValue = 1,
				endValue = 2,
				description = {
					[1] = "Find the mystic root under the city of Banuta and bring it to Tereban.",
					[2] = "The magic root was delievered to Tereban."
				}
			},
			{
				name = "The Magic Shield - Iron",
				storageId = Storage.FathersBurdenQuest.Iron,  -- 50212
				startValue = 1,
				endValue = 2,
				description = {
					[1] = "Find some old iron in the mines of Kazordoon for Tereban. Don't get lost - start searching close to the city.",
					[2] = "Tereban got the old iron he required."
				}
			},
			{
				name = "The Magic Shield - Scale",
				storageId = Storage.FathersBurdenQuest.Scale,  -- 50213
				startValue = 1,
				endValue = 2,
				description = {
					[1] = "Find the dragon Glitterscale in the caves north of Thais and take its scale to Tereban.",
					[2] = "You handed the looted scale to Tereban."
				}
			}
		}
	})

	quest:register()
	print(">> Registered quest: A Father's Burden (9 missions)")
	return true
end

fathersBurden:register()
