-- Secret Service Quest
-- Converted from quests.xml to Lua
-- Storage keys defined in data/lib/core/storages.lua under Storage.secretService

local secretServiceQuest = GlobalEvent("SecretServiceQuestStart")

function secretServiceQuest.onStartup()
	local quest = Game.createQuest("Secret Service", {
		storageId = Storage.secretService.Quest,  -- 12550
		storageValue = 1,
		endValue = 15,
		missions = {
			-- TBI (Thais Bureau of Investigation) - Chester missions
			{
				name = "Mission 1: From Thais with Love",
				storageId = 12551,
				startValue = 1,
				endValue = 3,
				description = {
					[1] = "Your first mission is to deliver a warning to the Venoreans. Get a fire bug from Liberty Bay and set their shipyard on fire.",
					[2] = "You have set the Venoreans shipyard on fire, report back to Chester!",
					[3] = "You have reported back that you have completed your mission, ask Chester for a new mission!"
				}
			},
			-- AVIN (A Venetian Intelligence Network) - Uncle missions
			{
				name = "Mission 1: For Your Eyes Only",
				storageId = 12552,
				startValue = 1,
				endValue = 4,
				description = {
					[1] = "Your first task is to deliver a letter to Gamel in thais, If he is a bit reluctant, be persuasive.",
					[2] = "Gamel sent his thugs on you, defeat them and deliver the letter to Gamel!",
					[3] = "After defeating Gamel's thugs, he found you to be persuasive enough to accept the letter. Report back to Uncle!",
					[4] = "You have reported back that you have completed your task. Ask Uncle for a new mission!"
				}
			},
			-- CGB (Carlin Governmental Bureau) - Emma missions
			{
				name = "Mission 1: Borrowed Knowledge",
				storageId = 12553,
				startValue = 1,
				endValue = 2,
				description = {
					[1] = "Emma has requested that you steal a Nature Magic Spellbook in the Edron academy.",
					[2] = "You have delivered the Nature Magic Spellbook to Emma, ask her for a new mission!"
				}
			},
			-- TBI Mission 2
			{
				name = "Mission 2: Operation Green Claw",
				storageId = 12554,
				startValue = 1,
				endValue = 2,
				description = {
					[1] = "Your next mission is to find some information about one of their missing agents in The Green Claw Swamp.",
					[2] = "You have delivered the Black Knight's notes to Chester, ask him for a new mission!"
				}
			},
			-- AVIN Mission 2
			{
				name = "Mission 2: A File Between Friends",
				storageId = 12555,
				startValue = 1,
				endValue = 2,
				description = {
					[1] = "Your next task is to retrieve a file named AH-X17L89.",
					[2] = "You have delivered the file named AH-X17L89 to Uncle, ask him for a new mission!"
				}
			},
			-- CGB Mission 2
			{
				name = "Mission 2: Codename:Lumberjack",
				storageId = 12556,
				startValue = 1,
				endValue = 2,
				description = {
					[1] = "Emma has requested that you retrieve a Rotten Heart of a Tree from the Black Knight Villa in Greenclaw swamp north-west of Venore.",
					[2] = "You have delivered the Rotten Heart of a Tree to Emma, ask her for a new mission!"
				}
			},
			-- TBI Mission 3
			{
				name = "Mission 3: Treachery in Port Hope",
				storageId = 12557,
				startValue = 1,
				endValue = 3,
				description = {
					[1] = "Your next mission is to retrieve some evidence that the traders in Port Hope are up to no good!",
					[2] = "You have found the evidence, report back to Chester!",
					[3] = "You have reported back that you have completed your mission, ask Chester for a new mission!"
				}
			},
			-- AVIN Mission 3
			{
				name = "Mission 3: What Men are Made of",
				storageId = 12558,
				startValue = 1,
				endValue = 4,
				description = {
					[1] = "Your next task is to bring a barrel of beer to the Secret Tavern in the sewers of Carlin.",
					[2] = "On your way to the Secret Tavern in the sewers you were attacked by amazons trying to stop you! Deliver the barrel of beer to Karl.",
					[3] = "You have delivered the barrel of beer to Karl, report back to Uncle!",
					[4] = "You have reported back that you have completed your task, ask Uncle for a new mission!"
				}
			},
			-- CGB Mission 3
			{
				name = "Mission 3: Rust in Peace",
				storageId = 12559,
				startValue = 1,
				endValue = 3,
				description = {
					[1] = "Emma has requested that you damage the Ironhouse of Venore, use the Case of Rust Bugs on the keyhole in the cellar of the ironhouse.",
					[2] = "The bugs are at work! Report back to Emma.",
					[3] = "You have reported back that you have completed your mission, ask her for a new mission!"
				}
			},
			-- TBI Mission 4
			{
				name = "Mission 4: Objective Hellgate",
				storageId = 12560,
				startValue = 1,
				endValue = 2,
				description = {
					[1] = "Your next mission is to investigate for some documents in Hellgate.",
					[2] = "You have delivered the documents to Chester, ask him for a new mission!"
				}
			},
			-- AVIN Mission 4
			{
				name = "Mission 4: Pawn Captures Knight",
				storageId = 12561,
				startValue = 1,
				endValue = 3,
				description = {
					[1] = "Your next task is to travel to the Black Knight's Villa and kill the Black Knight!",
					[2] = "You have killed the Black Knight, report back to Uncle!",
					[3] = "You have reported back that you have completed your task, ask Uncle for a new mission!"
				}
			},
			-- CGB Mission 4
			{
				name = "Mission 4: Plot for A Plan",
				storageId = 12562,
				startValue = 1,
				endValue = 2,
				description = {
					[1] = "Emma has requested that you retrieve the Building Plans for a ship from the Venore shipyard.",
					[2] = "You have delivered the Building Plans to Emma, ask her for a new mission!"
				}
			},
			-- TBI Mission 5
			{
				name = "Mission 5: Coldfinger",
				storageId = 12563,
				startValue = 1,
				endValue = 3,
				description = {
					[1] = "Your next mission is to travel to the southern barbarians camp and place false evidence!",
					[2] = "You have placed the false evidence! Report back to Chester.",
					[3] = "You have reported back that you have completed your mission, ask Chester for a new mission!"
				}
			},
			-- AVIN Mission 5
			{
				name = "Mission 5: A Cryptic Mission",
				storageId = 12564,
				startValue = 1,
				endValue = 2,
				description = {
					[1] = "Your next task is to travel to the Isle of the Kings and find a ring.",
					[2] = "You have delivered the ring to Uncle, ask him for a new mission!"
				}
			},
			-- CGB Mission 5
			{
				name = "Mission 5: No Admittance",
				storageId = 12565,
				startValue = 1,
				endValue = 2,
				description = {
					[1] = "Emma has requested that you find some hints in the ruins of Dark Cathedral.",
					[2] = "You have delivered the Suspicious Documents to Emma, ask her for a new mission!"
				}
			},
			-- TBI Mission 6
			{
				name = "Mission 6: The Weakest Spot",
				storageId = 12585,
				startValue = 1,
				endValue = 3,
				description = {
					[1] = "Your next mission is to disguise yourself as an amazon and destroy a beer casket in the north-east corner in the cellar of Svargrond's Tavern.",
					[2] = "You have succesfully destroyed the beer casket disguised as an amazon, report back to Chester!",
					[3] = "You have reported back that you have completed your mission, ask Chester for a new mission!"
				}
			},
			-- AVIN Mission 6
			{
				name = "Mission 6: A Little Bribe Won't Hurt",
				storageId = 12567,
				startValue = 1,
				endValue = 3,
				description = {
					[1] = "Your next task is to bribe a barbarian in the large barbarian camp with a weapons crate.",
					[2] = "You have bribed Freezhild with the weapons create! Report back to Uncle.",
					[3] = "You have reported back that you have completed your task, ask Uncle for a new mission!"
				}
			},
			-- CGB Mission 6
			{
				name = "Mission 6: News From the Past",
				storageId = 12568,
				startValue = 1,
				endValue = 2,
				description = {
					[1] = "Emma has requested that you go to the Isle of the Kings and retrieve a book.",
					[2] = "You have delivered the book to Emma, ask her for a new mission!"
				}
			},
			-- Mission 7 (shared final mission)
			{
				name = "Mission 7: Licence to Kill",
				storageId = 12569,
				startValue = 1,
				endValue = 2,
				description = {
					[1] = "A Mad Technomancer in Kazordoon is trying to blackmail the city! Kill him and bring back his beard as proof.",
					[2] = "You have reported back that you have completed your mission, you are now a Special Agent!"
				}
			}
		}
	})

	quest:register()
	print(">> Registered quest: Secret Service (19 missions)")
	return true
end

secretServiceQuest:register()
