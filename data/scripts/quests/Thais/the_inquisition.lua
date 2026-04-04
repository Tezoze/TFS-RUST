-- The Inquisition Quest
-- Converted from quests.xml to Lua
-- Quest NPC: Henricus (The Lord Inquisitor)
-- Storage keys defined in data/lib/core/storages.lua under Storage.TheInquisition

local theInquisition = GlobalEvent("TheInquisitionQuest")

function theInquisition.onStartup()
    local quest = Game.createQuest("The Inquisition", {
        storageId = Storage.TheInquisition.Questline,  -- 12160
        storageValue = 2,
        missions = {
            {
                name = "Mission 1: Interrogation",
                storageId = Storage.TheInquisition.Mission01,  -- 12161
                startValue = 1,
                endValue = 7,
                description = {
                    [1] = "Your mission is to investigate the 5 guards in Thais regarding the Heretic behavior. Tim, Kulag, Grof, Miles and Walter are their names. If you do well you see a holy sprite on you.",
                    [2] = "You investigated 1 of 5 guards in Thais.",
                    [3] = "You investigated 2 of 5 guards in Thais.",
                    [4] = "You investigated 3 of 5 guards in Thais.",
                    [5] = "You investigated 4 of 5 guards in Thais.",
                    [6] = "You investigated 5 of 5 guards in Thais. Get back to Thais and report your mission to Henricus.",
                    [7] = "You investigated all guards in Thais."
                }
            },
            {
                name = "Mission 2: Eclipse",
                storageId = Storage.TheInquisition.Mission02,  -- 12162
                startValue = 1,
                endValue = 3,
                description = {
                    [1] = "Henricus tells you to get The Witches' Grimoire, he sends you to Femor Hills where you can fly to the witches' mountain, say Eclipse to Uzon and he will take you there. Use the vial of holy water that he gives you on the big cauldron and open",
                    [2] = "Find the witches' grimoire and bring it to Henricus.",
                    [3] = "You already brought the witches' grimoire to Henricus."
                }
            },
            {
                name = "Mission 3: Vampire Hunt",
                storageId = Storage.TheInquisition.Mission03,  -- 12163
                startValue = 1,
                endValue = 6,
                description = {
                    [1] = "Henricus wants you to find the Dwarfish Vampire Hunter, Storkus, located west of the Dwarf Bridge. It's good idea bring your 20 Vampire Dusts with you to save some time.",
                    [2] = "Go Back to Storkus the Dwarf and ask for Mission.",
                    [3] = "Now Storkus wants you to kill a vampire lord, The Count in the Green Claw Swamp, The Count is located near to the Blood Herb Quest. To summon The Count, you must use the coffin in the center of the room. Kill it and bring The Ring of the Coun",
                    [4] = "Kill The Count and bring his ring to Storkus the Dwarf and ask for Mission.",
                    [5] = "Return to Henricus and tell him that you finished your job here.",
                    [6] = "Get back to Thais and report your mission to Henricus."
                }
            },
            {
                name = "Mission 4: The Haunted Ruin",
                storageId = Storage.TheInquisition.Mission04,  -- 12164
                startValue = 1,
                endValue = 3,
                description = {
                    [1] = "Henricus will gave you a Special Flask (vial of holy water). Go to Liberty Bay and use the vial on an old house. Use this vial of holy water on that spot to drive out the evil being.",
                    [2] = "Kill the Pirate Ghost and get back to Thais and report your mission to Henricus.",
                    [3] = "You already cleaned the abandoned and haunted house in Liberty, ask Henricus for mission."
                }
            },
            {
                name = "Mission 5: Essential Gathering",
                storageId = Storage.TheInquisition.Mission05,  -- 12165
                startValue = 1,
                endValue = 3,
                description = {
                    [1] = "Henricus wants 20 Demonic Essences as a proof of your accomplishments.",
                    [2] = "Now ask Henricus for outfit to get the Demon Hunter Outfit.",
                    [3] = "You got the Demon Hunter Outfit! Ask Henricus for mission to unlock more addons."
                }
            },
            {
                name = "Mission 6: The Demon Ungreez",
                storageId = Storage.TheInquisition.Mission06,  -- 12166
                startValue = 1,
                endValue = 3,
                description = {
                    [1] = "Henricus wants you to kill a demon called Ungreez. Head to Edron Hero Cave and go down a few levels.",
                    [2] = "You killed Ungreez, report your mission to Henricus.",
                    [3] = "You got the the first addon of Demon Hunter Outfit! Ask Henricus for mission to unlock more addons."
                }
            },
            {
                name = "Mission 7: The Shadow Nexus",
                storageId = Storage.TheInquisition.Mission07,  -- 12167
                startValue = 1,
                endValue = 5,
                description = {
                    [1] = "Your mission is to go to the Demon Forge and slay seven of The Ruthless Seven Minions. The Demon Forge is located in the Edron Hero Cave, through a portal after the Vampire Shield Quest.",
                    [2] = "You destroyed the shadow nexus! Get back to Thais and report your mission to Henricus.",
                    [3] = "Now ask to Henricus for a outfit. He will give you the 2nd addon of the Demon Hunter Outfits.",
                    [4] = "You got the the second addon of Demon Hunter Outfit! Go now to the reward room and choose one wisely!",
                    [5] = "You have completed The Inquisition Quest! You can now buy the Blessing of the Inquisition!"
                }
            }
        }
    })

    quest:register()
    print(">> Registered quest: The Inquisition (7 missions)")
    return true
end

theInquisition:register()
