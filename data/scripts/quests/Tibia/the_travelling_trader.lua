local theTravellingTraderQuest = GlobalEvent("TheTravellingTraderQuestStart")

function theTravellingTraderQuest.onStartup()
	local quest = Game.createQuest("The Travelling Trader Quest", {
		storageId = 51201,
		storageValue = 1,
		missions = {
			{
				name = "Mission 1: Trophy",
				storageId = 51202,
				startValue = 1,
				endValue = 2,
				description = {
					[1] = "Your first mission for becoming a recognized trader is to bring the traveling salesman Rashid a Deer Trophy.",
					[2] = "You have brought a Deer Trophy to Rashid and completed the mission."
				}
			},
			{
				name = "Mission 2: Delivery",
				storageId = 51203,
				startValue = 1,
				endValue = 5,
				description = {
					[1] = "Your mission is to get the package from Willard the weapon dealer at Edron.",
					[2] = "Willard forgot to pick it up from Snake Eye at Outlaw Camp. So he wants you to go and pick it up from Snake Eye.",
					[3] = "Take the package just next door.",
					[4] = "Now bring back the package to Rashid.",
					[5] = "You have brought the package to Rashid and completed the mission."
				}
			},
			{
				name = "Mission 3: Cheese",
				storageId = 51204,
				startValue = 1,
				endValue = 3,
				description = {
					[1] = "Rashid wants you to pick his special order from Miraia in Darashia. But you have to be quick, Scarab cheese can rot really fast in high temperature.",
					[2] = "Now quickly bring back the Scarab cheese to Rashid.",
					[3] = "You have brought the Scarab cheese to Rashid and completed the mission."
				}
			},
			{
				name = "Mission 4: Vase",
				storageId = 51205,
				startValue = 1,
				endValue = 3,
				description = {
					[1] = "Rashid have ordered a special elven vase from Briasol in Ab'Dendriel. He asks you to buy it from Briasol and bring it back.\nBut you should be carefully, since the vase is very fragile.",
					[2] = "Now carefully bring the vase back to Rashid.",
					[3] = "You have brought the vase to Rashid and completed the mission."
				}
			},
			{
				name = "Mission 5: Make a deal",
				storageId = 51206,
				startValue = 1,
				endValue = 3,
				description = {
					[1] = "This time, Rashid is testing your trading skills to buy a Crimson Sword from Uzgod. But it have to be less than 400 gold coins and the quality has to be perfect.",
					[2] = "Now bring the sword back to Rashid.",
					[3] = "You have brought the Crimson Sword to Rashid and completed the mission."
				}
			},
			{
				name = "Mission 6: Goldfish",
				storageId = 51207,
				startValue = 1,
				endValue = 2,
				description = {
					[1] = "Rashid wants you to bring him a Goldfish Bowl.",
					[2] = "You have brought a Goldfish Bowl to Rashid and completed the mission."
				}
			},
			{
				name = "Mission 7: Declare",
				storageId = 51208,
				startValue = 1,
				endValue = 1,
				description = {
					[1] = "Rashid has declared you as one of his recognized traders, and now you are able to trade with him anytime."
				}
			}
		}
	})

	quest:register()
	print(">> Registered quest: The Travelling Trader Quest")
	return true
end

theTravellingTraderQuest:register()
