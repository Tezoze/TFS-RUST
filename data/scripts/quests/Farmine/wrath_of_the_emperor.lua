-- Wrath of the Emperor Quest
-- Converted from quests.xml to Lua
-- Storage keys defined in data/lib/core/storages.lua under Storage.WrathoftheEmperor

local wrathOfTheEmperorQuest = GlobalEvent("WrathOfTheEmperorQuestStart")

function wrathOfTheEmperorQuest.onStartup()
	local quest = Game.createQuest("Wrath of the Emperor", {
		storageId = Storage.WrathoftheEmperor.Questline,  -- 30094
		storageValue = 1,
		missions = {
			{
				name = "Mission 01: Catering the Lions Den",
				storageId = Storage.WrathoftheEmperor.Mission01,  -- 30095
				startValue = 1,
				endValue = 3,
				description = {
					[1] = "You must bring Zalamon 3 nails and a piece of wood so that he can make a Marked Crate for you.",
					[2] = "Go to the tunnel in eastern Muggy Plains and reach the other side. Try to hide in the dark and avoid being seen at all by using the crate. After that you need to find the rebel hideout and talk to their leader Chartan.",
					[3] = "You found the leader of the rebel Chartan and reported him about Zalamon. Ask him for new mission!"
				}
			},
			{
				name = "Mission 02: First Contact",
				storageId = Storage.WrathoftheEmperor.Mission02,  -- 30096
				startValue = 1,
				endValue = 3,
				description = {
					[1] = "Chartan needs you to reactivate the teleport to the Muggy Plains. Head downstairs and into the temple and craft material to repair the teleport. To do this you will need some tools to improvise.",
					[2] = "As you give the coal into the pool the corrupted fluid begins to dissolve, leaving purified, refreshing water. The teleporter is reactivated. Report back to Chartan.",
					[3] = "Report back to Zalamon for the next mission."
				}
			},
			{
				name = "Mission 03: The Keeper",
				storageId = Storage.WrathoftheEmperor.Mission03,  -- 30097
				startValue = 1,
				endValue = 3,
				description = {
					[1] = "Zalamon gives you a Flask of Plant Poison to destroy plants in the garden of the Emperor to lure out and kill The Keeper to get his tail. The garden is southeast of the rebel hideout.",
					[2] = "You killed the Keeper and got his tail. Bring it to Zalamon.",
					[3] = "You brought the tail of the Keeper to Zalamon and completed the mission. Ask for the next mission."
				}
			},
			{
				name = "Mission 04: Sacrament of the Snake",
				storageId = Storage.WrathoftheEmperor.Mission04,  -- 30098
				startValue = 1,
				endValue = 3,
				description = {
					[1] = "Zalamon now wants you to go to Deeper Banuta and get an Ancient Sceptre that will help in the fight against the emperor. On each floor under Deeper Banuta you collect a sceptre part from a Ghost of a Priest. On the 4th and final floor you nee",
					[2] = "After you've assembled the Snake Sceptre and fought your way back out, head back to Zalamon and give it to him.",
					[3] = "You brought the Snake Sceptre to Zalamon and completed the mission. Ask for the next mission."
				}
			},
			{
				name = "Mission 05: New in Town",
				storageId = Storage.WrathoftheEmperor.Mission05,  -- 30099
				startValue = 1,
				endValue = 3,
				description = {
					[1] = "Zalamon tells you that you have to go inside the city. From the rebel hideout go out to the gray road and follow it to the southwest. Find the Gate Guardian and ask him for a mission to enter the city.",
					[2] = "Now you only have to walk west until you find Zlak inside the big green building.",
					[3] = "You went deep inside the city to find Zlak and completed the mission. Ask for the next mission."
				}
			},
			{
				name = "Mission 06: The Office Job",
				storageId = Storage.WrathoftheEmperor.Mission06,  -- 30100
				startValue = 0,
				endValue = 4,
				description = {
					[0] = "Kill four Magistrati in the office building. then report back to Zlak. you have kiled 0 Magistrati so far.",
					[1] = "Kill four Magistrati in the office building. then report back to Zlak. you have kiled 1 Magistrati so far.",
					[2] = "Kill four Magistrati in the office building. then report back to Zlak. you have kiled 2 Magistrati so far.",
					[3] = "Kill four Magistrati in the office building. then report back to Zlak. you have kiled 3 Magistrati so far.",
					[4] = "Report back to Zlak. you have kiled 4 Magistrati."
				}
			},
			{
				name = "Mission 07: A Noble Cause",
				storageId = Storage.WrathoftheEmperor.Mission07,  -- 30101
				startValue = 0,
				endValue = 6,
				description = {
					[0] = "Kill six lizard nobles in the office building. then report back to Zlak. you have kiled 0 lizard noble so far.",
					[1] = "Kill six lizard nobles in the office building. then report back to Zlak. you have kiled 1 lizard noble so far.",
					[2] = "Kill six lizard nobles in the office building. then report back to Zlak. you have kiled 2 lizard noble so far.",
					[3] = "Kill six lizard nobles in the office building. then report back to Zlak. you have kiled 3 lizard noble so far.",
					[4] = "Kill six lizard nobles in the office building. then report back to Zlak. you have kiled 4 lizard noble so far.",
					[5] = "Kill six lizard nobles in the office building. then report back to Zlak. you have kiled 5 lizard noble so far.",
					[6] = "Report back to Zlak. you have kiled 6 lizard noble so far."
				}
			},
			{
				name = "Mission 08: Uninvited Guests",
				storageId = Storage.WrathoftheEmperor.Mission08,  -- 30102
				startValue = 1,
				endValue = 2,
				description = {
					[1] = "Your mission is to reach your rebel contact Zizzle in the imperial palace. You need to take the old escape tunnel that leads from the abandoned basement in the north of the ministry to a lift that ends somewhere in the palace.",
					[2] = "You have reached your rebel contact Zizzle in the imperial palace."
				}
			},
			{
				name = "Mission 09: The Sleeping Dragon",
				storageId = Storage.WrathoftheEmperor.Mission09,  -- 30103
				startValue = 1,
				endValue = 2,
				description = {
					[1] = "To enter the inner realms of the Emperor you need to free the mind of a dragon. An interdimensional potion will help you to enter this dream and unleash his consciousness.",
					[2] = "You travelled through the Sleeping Dragon dreams and freed his mind."
				}
			},
			{
				name = "Mission 10: A Message of Freedom",
				storageId = Storage.WrathoftheEmperor.Mission10,  -- 30104
				startValue = 1,
				endValue = 6,
				description = {
					[1] = "After solving the riddle, and talking again to the Sleeping Dragon you got a Spiritual Charm. Report back to Zizzle.",
					[2] = "You possess the key to enter the inner realms of the emperor. Start with the one in the north-west and work your way clockwise trough the room and kill those manifestation. Then use your sceptre on the remain to destroy the emperor's influenc",
					[3] = "You possess the key to enter the inner realms of the emperor. You destroyed 1 of 4 emperor's influences.",
					[4] = "You possess the key to enter the inner realms of the emperor. You destroyed 2 of 4 emperor's influences.",
					[5] = "You possess the key to enter the inner realms of the emperor. You destroyed 3 of 4 emperor's influences.",
					[6] = "You possess the key to enter the inner realms of the emperor. You destroyed all emperor's influences."
				}
			},
			{
				name = "Mission 11: Payback Time",
				storageId = Storage.WrathoftheEmperor.Mission11,  -- 30105
				startValue = 0,
				endValue = 2,
				description = {
					[1] = "Your Mission is to kill Zalamon. Step into the teleporter to confront him. Finally use your sceptre on the death body.",
					[2] = "Go back to Awareness Of The Emperor and report him your success!"
				}
			},
			{
				name = "Mission 12: Just Rewards",
				storageId = Storage.WrathoftheEmperor.Mission12,  -- 30106
				startValue = 0,
				endValue = 1,
				description = {
					[0] = "The Emperor has promised you wealth beyond measure. Go claim it in the ministry.",
					[1] = "You completed this Quest!"
				}
			}
		}
	})

	quest:register()
	print(">> Registered quest: Wrath of the Emperor (12 missions)")
	return true
end

wrathOfTheEmperorQuest:register()
