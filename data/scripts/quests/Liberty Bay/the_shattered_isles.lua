-- The Shattered Isles Quest
-- Converted from quests.xml to Lua
-- Storage keys defined in data/lib/core/storages.lua under Storage.TheShatteredIsles

local shatteredIslesQuest = GlobalEvent("ShatteredIslesQuestStart")

function shatteredIslesQuest.onStartup()
	local quest = Game.createQuest("The Shattered Isles", {
		storageId = Storage.TheShatteredIsles.Questline,  -- 12175
		storageValue = 1,
		missions = {
			{
				name = "A Djinn in Love",
				storageId = Storage.TheShatteredIsles.ADjinnInLove,  -- 12179
				startValue = 1,
				endValue = 5,
				description = {
					[1] = "You need to return to Marina and ask her for a date with Ocelus.",
					[2] = "You need to return to Ocelus with the bad news.",
					[3] = "Ocelus told you to get a poem for him, if you didn't buy one already, head to Ab'Dendriel and buy a Love Poem from Elvith.",
					[4] = "You need to go recite the poem to Marina and impress her with the Djinn's romantic and poetic abilities.",
					[5] = "After reciting the poem to Marina, she decided to date Ocelus and release Ray Striker from her spell."
				}
			},
			{
				name = "A Poem for the Mermaid",
				storageId = Storage.TheShatteredIsles.APoemForTheMermaid,  -- 12177
				startValue = 1,
				endValue = 3,
				description = {
					[1] = "You need to find the man-stealing mermaid and try to break her spell over poor Raymond, the mermaid Marina is near the northern coast of the island.",
					[2] = "You discovered that she does in fact have a spell on him, and will not release him unless someone better comes along.",
					[3] = "You are a true master in reciting love poems now. No mermaid will be able to resist if you ask for a date!"
				}
			},
			{
				name = "Shipwrecked",
				storageId = Storage.TheShatteredIsles.Shipwrecked,  -- 12176
				startValue = 1,
				endValue = 2,
				description = {
					[1] = "Oh no! A tropical storm wrecked your ship and you washed up on the volcanic island of Goroma. Captain Jack Fate's ship is damaged and he needs 30 pieces of wood to repair it before you can return to Liberty Bay. You can bring him wood pieces incrementally.",
					[2] = "You have collected 30 pieces of wood and helped Captain Jack Fate repair his ship. He will now take you back to Liberty Bay whenever you ask for a passage."
				}
			},
			{
				name = "Access to Goroma",
				storageId = Storage.TheShatteredIsles.AccessToGoroma,  -- 12178
				startValue = 1,
				endValue = 1,
				description = {
					[1] = "After helping Jack Fate to collect the 30 woodpieces, Jack Fate in Liberty Bay will bring you to Goroma."
				}
			},
			{
				name = "Access to Laguna Island",
				storageId = Storage.TheShatteredIsles.AccessToLagunaIsland,  -- 12180
				startValue = 1,
				endValue = 1,
				description = {
					[1] = "After arranging a date for Marina and Ocelus, you are allowed to use Marina's sea turtles. They will bring you to the idyllic Laguna Islands."
				}
			},
			{
				name = "Access to Meriana",
				storageId = Storage.TheShatteredIsles.AccessToMeriana,  -- 12181
				startValue = 1,
				endValue = 1,
				description = {
					[1] = "After earning the trust of the governor's daughter Eleonore, Captain Waverider in Liberty Bay will bring you to Meriana if you tell him the secret codeword 'peg leg'."
				}
			},
			{
				name = "The Counterspell",
				storageId = Storage.TheShatteredIsles.TheCounterspell,  -- 12182
				startValue = 0,
				endValue = 4,
				description = {
					[0] = "Deliver Dragah's spellbook to Chondur so he can teach you about counterspells.",
					[1] = "You have begun Chondur's ritual. Bring him a fresh dead chicken so that he can begin to create a counterspell which will allow you to pass the magical barrier on Goroma.",
					[2] = "You have begun Chondur's ritual. Bring him a fresh dead rat so that he can continue creating a counterspell which will allow you to pass the magical barrier on Goroma.",
					[3] = "You have begun Chondur's ritual. Bring him a fresh dead black sheep so that he can complete his counterspell which will allow you to pass the magical barrier on Goroma.",
					[4] = "You may pass the energy barrier on Goroma. The counterspell Chondur created for you with his ritual will allow you to withstand the evil magic of the cultist."
				}
			},
			{
				name = "The Errand",
				storageId = Storage.TheShatteredIsles.TheErrand,  -- 12185
				startValue = 1,
				endValue = 4,
				description = {
					[1] = "You told Eleonore to run a small errand. Deliver the 200 gold pieces she gave to the herbalist Charlotta in the south-western part of Liberty Bay.",
					[2] = "You delivered the gold to Charlotta. Return to Eleonore and tell her - Errand",
					[3] = "Tell to Eleonore the secret password: peg leg",
					[4] = "Contact Captain Waverider, the old fisherman, and tell him the secret word 'peg leg'."
				}
			},
			{
				name = "The Governor's Daughter",
				storageId = Storage.TheShatteredIsles.TheGovernorDaughter,  -- 12184
				startValue = 1,
				endValue = 3,
				description = {
					[1] = "You promised to find Eleonore's lost ring. She told you that a parrot stole it from her dressing table and flew to the nearby mountains. You might need a rake to retrieve the ring.",
					[2] = "You found the ring. Return it to Eleonore.",
					[3] = "You returned the ring to Eleonore."
				}
			}
		}
	})

	quest:register()
	print(">> Registered quest: The Shattered Isles (9 missions)")
	return true
end

shatteredIslesQuest:register()
