-- The New Frontier Quest
-- Converted from quests.xml to Lua
-- Storage keys defined in data/lib/core/storages.lua under Storage.TheNewFrontier

local theNewFrontier = GlobalEvent("TheNewFrontierQuest")

function theNewFrontier.onStartup()
    local quest = Game.createQuest("The New Frontier", {
        storageId = Storage.TheNewFrontier.Questline,  -- 12130
        storageValue = 1,
        missions = {
            {
                name = "Mission 01: New Land",
                storageId = Storage.TheNewFrontier.Mission01,  -- 12131
                startValue = 1,
                endValue = 3,
                description = {
                    [1] = "Ongulf sends you to explore the passage to the east of farmine.",
                    [2] = "You have found the passage through the mountains and can report Ongulf about your success.",
                    [3] = "You already reported Ongulf your success! Ask Ongulf for new mission!"
                }
            },
            {
                name = "Mission 02: From Kazordoon With Love",
                storageId = Storage.TheNewFrontier.Mission02,  -- 12132
                startValue = 1,
                endValue = 6,
                description = {
                    [1] = "Ongulf will tell you that he needs to send a message to the dwarves in Kazordoon. Travel there and then walk to the Dwarf Mines to the west. Cross the two rivers and find Melfar.",
                    [2] = "Melfar gave you a Flask with Beaver Bait that you must use on the 3 trees (Trees will be marked on your map).",
                    [3] = "You marked 1 of 3 trees with Beaver Bait in near the Dwarf Mines",
                    [4] = "You marked 2 of 3 trees with Beaver Bait in near the Dwarf Mines",
                    [5] = "You marked all 3 trees with Beaver Bait in near the Dwarf Mines. Return to talk to Melfar before heading back to Farmine.",
                    [6] = "Return to Ongulf and report your mission!"
                }
            },
            {
                name = "Mission 03: Strangers in the Night",
                storageId = Storage.TheNewFrontier.Mission03,  -- 12133
                startValue = 1,
                endValue = 3,
                description = {
                    [1] = "You need to find the tribe which lives in the mountains. Find some vines in this mountain, climb up there and find out who is spying on us!",
                    [2] = "Return to Ongulf and report your mission about primitive humans.",
                    [3] = "You already reported Ongulf about primitive humans! Ask Ongulf for new mission!"
                }
            },
            {
                name = "Mission 04: The Mine Is Mine",
                storageId = Storage.TheNewFrontier.Mission04,  -- 12134
                startValue = 1,
                endValue = 2,
                description = {
                    [1] = "Head back to the levers and this time use the left one. Go North through the tunnel of Stone Golems. Kill the boss, Shard of Corruption at the end of the tunnel.",
                    [2] = "You killed the Shard of Corruption. Return to Ongulf and report your mission!"
                }
            },
            {
                name = "Mission 05: Getting Things Busy",
                storageId = Storage.TheNewFrontier.Mission05,  -- 12135
                startValue = 1,
                endValue = 7,
                description = {
                    [1] = "This mission consists of getting support from 6 people (King Tibianus in Thais, Leeland in Venore, Angus in Port Hope, Wyrdin in the Ivory Towers, Telas in Edron, Humgolf in Kazordoon) around Tibia.",
                    [2] = "You got support from 1 of 6 people: King Tibianus in Thais, Leeland in Venore, Angus in Port Hope, Wyrdin in the Ivory Towers, Telas in Edron and Humgolf in Kazordoon.",
                    [3] = "You got support from 2 of 6 people: King Tibianus in Thais, Leeland in Venore, Angus in Port Hope, Wyrdin in the Ivory Towers, Telas in Edron and Humgolf in Kazordoon.",
                    [4] = "You got support from 3 of 6 people: King Tibianus in Thais, Leeland in Venore, Angus in Port Hope, Wyrdin in the Ivory Towers, Telas in Edron and Humgolf in Kazordoon.",
                    [5] = "You got support from 4 of 6 people: King Tibianus in Thais, Leeland in Venore, Angus in Port Hope, Wyrdin in the Ivory Towers, Telas in Edron and Humgolf in Kazordoon.",
                    [6] = "You got support from 5 of 6 people: King Tibianus in Thais, Leeland in Venore, Angus in Port Hope, Wyrdin in the Ivory Towers, Telas in Edron and Humgolf in Kazordoon.",
                    [7] = "You got support from all needed people to help farmine! Return to Ongulf and report your mission!"
                }
            },
            {
                name = "Mission 06: Days Of Doom",
                storageId = Storage.TheNewFrontier.Mission06,  -- 12136
                startValue = 1,
                endValue = 3,
                description = {
                    [1] = "Go to Curos, leader of the Orcs, in the middle of the Zao Steppe in the north-east area of an Orc camp. Ask for a mission and accept it.",
                    [2] = "Enter the ring of battle, close to Curos quarter. Face Mooh'Tah Master in a battle and survive for two minutes. Return to him after you have passed this test.",
                    [3] = "You passed the test of Curos. Return to Ongulf and report your mission!"
                }
            },
            {
                name = "Mission 07: Messengers Of Peace",
                storageId = Storage.TheNewFrontier.Mission07,  -- 12137
                startValue = 1,
                endValue = 3,
                description = {
                    [1] = "You now have to contact the Lizards, the real rulers of Zao. Find them, contact them, talk to them, scare them, bribe them, whatever.",
                    [2] = "You have been trapped! Find a way out!",
                    [3] = "You found the Lizards!"
                }
            },
            {
                name = "Mission 08: An Offer You Can't Refuse",
                storageId = Storage.TheNewFrontier.Mission08,  -- 12138
                startValue = 1,
                endValue = 2,
                description = {
                    [1] = "Take the boat at the northern of Dragonblaze Peaks to tournament Isle. Ask Zurak for a passage. There you'll learn anything you need to know about the great tournament. Ask there Chrak for the battle.",
                    [2] = "You agreed the Offer."
                }
            },
            {
                name = "Mission 09: Mortal Combat",
                storageId = Storage.TheNewFrontier.Mission09,  -- 12139
                startValue = 1,
                endValue = 3,
                description = {
                    [1] = "You have to go into the Arena with another player, because 2 players are needed, at the same time, to enter. Choose your companion wisely!",
                    [2] = "You have won! Report Chrak your mission about the battle.",
                    [3] = "Go back to Farmine and talk to Ongulf about your mission."
                }
            },
            {
                name = "Mission 10: New Horizons",
                storageId = Storage.TheNewFrontier.Mission10,  -- 12140
                startValue = 1,
                endValue = 1,
                description = {
                    [1] = "You now have permission to use the Magic Carpet on the mountain above Farmine, Cael now accepts more Tomes of Knowledge from you and you got the Warmaster Outfit!"
                }
            },
            {
                name = "Tome of Knowledge Counter",
                storageId = Storage.TheNewFrontier.TomeofKnowledge,  -- 12141
                startValue = 1,
                endValue = 12,
                description = {
                    [1] = "You brought the first Tome of Knowledge to Cael. He learnt more about the lizard culture. Pompan will sell you dragon tapestries from now on.",
                    [2] = "You brought the second Tome of Knowledge to Cael. He learnt more about the minotaur culture. Pompan will sell you minotaur backpacks from now on.",
                    [3] = "You brought the third Tome of Knowledge to Cael. He learnt more about the last stand of the draken culture. Esrik will sell you lizard weapon rack from now on.",
                    [4] = "You brought the fourth Tome of Knowledge to Cael. He learnt something interesting about a certain food that the lizardmen apparently prepare. Swolt will trade you a bunch of ripe rice for 10 rice balls from now on.",
                    [5] = "You brought the fifth Tome of Knowledge to Cael. He learnt more about the last stand of the lizards in the South, Zzaion. Pompan will sell you lizard backpacks from now on.",
                    [6] = "You brought the sixth Tome of Knowledge to Cael. He learnt a few things about the primitive human culture on this continent. Cael will sell you War Drums and Didgeridoo from now on.",
                    [7] = "You brought the seventh Tome of Knowledge to Cael. He learnt something interesting about the Zao steppe. Now you can use the snake teleport at the peak of the mountain.",
                    [8] = "You brought the eighth Tome of Knowledge to Cael. He learnt a few things about an illness. Now you can enter a corruption hole in the north-eastern end of Zao.",
                    [9] = "You brought the ninth Tome of Knowledge to Cael. He learnt something interesting about the Draken origin. Esrik will buy lizard equipment from you now.",
                    [10] = "You brought the tenth Tome of Knowledge to Cael. He learnt more about the last stand of the lizard dynasty. Now you can enter the Zao Palace. It is situated deep underground, below the mountain base of the Lizards.",
                    [11] = "You brought the eleventh Tome of Knowledge to Cael. He learnt something interesting about dragons and their symbolism. You can buy a Dragon Statue from NPC Cael after you bring him a Red Lantern.",
                    [12] = "You brought the twelfth Tome of Knowledge to Cael. He learnt something that reveals some of the power structures in Zao. Cael will now make a Dragon Throne for you after you bring him a Red Piece of Cloth. He will reward you with 5000 experience."
                }
            }
        }
    })

    quest:register()
    print(">> Registered quest: The New Frontier (11 missions)")
    return true
end

theNewFrontier:register()
