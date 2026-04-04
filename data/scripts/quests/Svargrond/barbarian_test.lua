-- Barbarian Test Quest
-- Converted from quests.xml to Lua
-- Storage keys defined in data/lib/core/storages.lua under Storage.BarbarianTest

local barbarianTest = GlobalEvent("BarbarianTestQuestStart")

function barbarianTest.onStartup()
	local quest = Game.createQuest("Barbarian Test Quest", {
		storageId = Storage.BarbarianTest.Questline,  -- 12190
		storageValue = 1,
		missions = {
			{
				name = "Barbarian Test 1: Barbarian Booze",
				storageId = Storage.BarbarianTest.Mission01,  -- 12191
				startValue = 1,
				endValue = 3,
				description = {
					[1] = "Talk to Sven about mead and give him a honeycomb. For each honeycomb you will be allowed 20 sips.",
					[2] = "Now drink from the bucket until you drink 10 sips in a row without passing out",
					[3] = "You have completed this Test! Talk to Sven about the mead."
				}
			},
			{
				name = "Barbarian Test 2: The Bear Hugging",
				storageId = Storage.BarbarianTest.Mission02,  -- 12192
				startValue = 1,
				endValue = 3,
				description = {
					[1] = "Sven gave you a Mead Horn. Fill it with mead from the bucket behind Sven (brown contents) and then proceed to the sleeping bear. The bear is inside a small ice cave in the north. Use the full mead horn on the bear so it becomes unconscious, t",
					[2] = "You hugging the bear! Go tell Sven that you hugged the bear!",
					[3] = "You have completed this Test!"
				}
			},
			{
				name = "Barbarian Test 3: The Mammoth Pushing",
				storageId = Storage.BarbarianTest.Mission03,  -- 12193
				startValue = 1,
				endValue = 3,
				description = {
					[1] = "Go to the north-west of Svargrond and find the Mammoth. Drink your three mugs of mead, stand in front of the Mammoth and push it. Just use it...",
					[2] = "You pushed the Mammoth! Go tell Sven that you pushed the Mammoth!",
					[3] = "You have completed this Test! You can now be a citizen of Svargrond!"
				}
			}
		}
	})

	quest:register()
	print(">> Registered quest: Barbarian Test Quest (3 missions)")
	return true
end

barbarianTest:register()
