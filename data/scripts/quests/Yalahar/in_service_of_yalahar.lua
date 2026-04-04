-- In Service of Yalahar Quest
-- Converted from quests.xml to Lua
-- Quest NPCs: Wyrdin, Palimuth, Azerus
-- Storage keys defined in data/lib/core/storages.lua under Storage.InServiceofYalahar and Storage.TheWayToYalahar

local inServiceOfYalahar = GlobalEvent("InServiceOfYalaharQuest")

function inServiceOfYalahar.onStartup()
    local quest = Game.createQuest("In Service of Yalahar", {
        storageId = Storage.TheWayToYalahar.QuestLine,  -- 30856 (CORRECTED)
        storageValue = 1,
        missions = {
            {
                name = "Mission 01: Something Rotten",
                storageId = Storage.InServiceofYalahar.Mission01,  -- 30205
                startValue = 1,
                endValue = 6,
                description = {
                    [1] = "Palimuth asked you to help with some sewer malfunctions. You will need a Crowbar, there are 4 places where you need to go marked with an X on your map.",
                    [2] = "You cleaned 1 pipe of 4 from the garbage.",
                    [3] = "You cleaned 2 pipes of 4 from the garbage.",
                    [4] = "You cleaned 3 pipes of 4 from the garbage.",
                    [5] = "You cleaned 4 pipes of 4 from the garbage. Go back to Palimuth and report your mission",
                    [6] = "You cleaned all pipes from the garbage! Go back to Palimuth and ask for mission."
                }
            },
            {
                name = "Mission 02: Watching the Watchmen",
                storageId = Storage.InServiceofYalahar.Mission02,  -- 30206
                startValue = 1,
                endValue = 8,
                description = {
                    [1] = "You have to find all 7 guards and give a report to them. You should start by Foreign Quarter or by Trade Quarter and follow the order of the path.",
                    [2] = "You reported to 1 of 7 guards",
                    [3] = "You reported to 2 of 7 guards",
                    [4] = "You reported to 3 of 7 guards",
                    [5] = "You reported to 4 of 7 guards",
                    [6] = "You reported to 5 of 7 guards",
                    [7] = "You reported to 6 of 7 guards",
                    [8] = "You reported to 7 of 7 guards! Go back to Palimuth and ask for mission."
                }
            },
            {
                name = "Mission 03: Death to the Deathbringer",
                storageId = Storage.InServiceofYalahar.Mission03,  -- 30207
                startValue = 1,
                endValue = 6,
                description = {
                    [1] = "Get the notes in Palimuths room and read them. Talk to Palimuth again when you've read the notes.",
                    [2] = "Talk to Azerus the Yalahari in the city centre to get your next mission.",
                    [3] = "Get the notes behind the Yalahari and read them. Talk to Azerus again and ask him for mission when you've read the notes.",
                    [4] = "Kill the three plague carriers in the alchemist quarter and retrieve The Alchemists' Formulas. When this is done, head back to either Palimuth (good side) or Yalahari (Azerus) (bad side).",
                    [5] = "Ask Palimuth for mission.",
                    [6] = "Ask Azerus the Yalahari for a mission."
                }
            },
            {
                name = "Mission 04: Good to be Kingpin",
                storageId = Storage.InServiceofYalahar.Mission04,  -- 30208
                startValue = 1,
                endValue = 6,
                description = {
                    [1] = "Ask Palimuth for mission.",
                    [2] = "For this mission you are asked to go to the Trade Quarter and negotiate or threaten Mr. West. Once again you will gain access to the mechanism although if you choose to help Palimuth you should go through the sewers.",
                    [3] = "You decided to help Palimuth, report him your mission.",
                    [4] = "You decided to help Azerus, report him your mission.",
                    [5] = "Get back to Azerus and report him your mission.",
                    [6] = "Ask Azerus for a mission."
                }
            },
            {
                name = "Mission 05: Food or Fight",
                storageId = Storage.InServiceofYalahar.Mission05,  -- 30209
                startValue = 1,
                endValue = 8,
                description = {
                    [1] = "Ask Palimuth for mission.",
                    [2] = "On this mission you are asked to find a druid by the name of Tamerin, on the Arena Quarter. You now have permission to use the gates mechanism.",
                    [3] = "The first is to bring Tamerin a flask of Animal Cure, you can buy this from Siflind on Nibelor (northeast of Svargrond).",
                    [4] = "Now you have to kill Morik the Gladiator and bring his helmet to Tamerin as proof.",
                    [5] = "Report back to Tamerin as he will listen to your request and you can now make your choice: Cattle for Palimuth (good side), Warbeasts for Yalahari (Azerus) (bad side). Then report the one you decided your mission.",
                    [6] = "You decided to help Palimuth, report him your mission.",
                    [7] = "You decided to help Azerus, report him your mission.",
                    [8] = "Ask Azerus for a mission."
                }
            },
            {
                name = "Mission 06: Frightening Fuel",
                storageId = Storage.InServiceofYalahar.Mission06,  -- 30210
                startValue = 1,
                endValue = 5,
                description = {
                    [1] = "Ask Palimuth for mission.",
                    [2] = "Yalahari (Azerus) orders you to travel to the Cemetery Quarter and find the Strange Carving. He gives you a Ghost Charm and tells you to charge it with the tormented souls of the ghosts there to be used as an energy source.",
                    [3] = "Good side: Go to Palimuth, ask him about your mission, and hand in the charm. Bad side: Ask about your mission to Yalahari (Azerus) and give it back.",
                    [4] = "Get back to Azerus and report him your mission.",
                    [5] = "Ask Azerus for a mission."
                }
            },
            {
                name = "Mission 07: A Fishy Mission",
                storageId = Storage.InServiceofYalahar.Mission07,  -- 30211
                startValue = 1,
                endValue = 5,
                description = {
                    [1] = "Ask Palimuth for mission.",
                    [2] = "Bad side: Yalahari (Azerus) will send you for a new mission to go to the Sunken Quarter and kill the Quara Leaders, Inky, Splasher and Sharptooth. Good side: Rather than fighting any Quara leaders Palimuth will instead send you to find the cause.",
                    [3] = "Get back to Palimuth and report him your mission.",
                    [4] = "You killed the Quara bosses. Ask Azerus for a mission.",
                    [5] = "Ask Azerus for a mission."
                }
            },
            {
                name = "Mission 08: Dangerous Machinations",
                storageId = Storage.InServiceofYalahar.Mission08,  -- 30212
                startValue = 1,
                endValue = 4,
                description = {
                    [1] = "Ask Palimuth for mission.",
                    [2] = "Bad side: For this mission the Yalahari requests you go to the Factory Quarter and find a pattern crystal, which will be used to supply weapons to help take control of the city. Good side: Palimuth will send you there to use the crystal to supply tools instead.",
                    [3] = "Get back to Azerus and report him your mission.",
                    [4] = "Ask Azerus for a mission."
                }
            },
            {
                name = "Mission 09: Decision",
                storageId = Storage.InServiceofYalahar.Mission09,  -- 30213
                startValue = 1,
                endValue = 2,
                description = {
                    [1] = "You now need to decide between supporting Palimuth or the Yalahari's goal. To choose Palimuth's good side go to him, and simply ask him for a mission. Likewise, to join the Yalahari (Azerus) (bad side) go to him and say the same.",
                    [2] = "You already decided!"
                }
            },
            {
                name = "Mission 10: The Final Battle",
                storageId = Storage.InServiceofYalahar.Mission10,  -- 30214
                startValue = 1,
                endValue = 5,
                description = {
                    [1] = "Palimuth told you that a circle of Yalahari is planning some kind of ritual. They plan to create a portal for some powerful demons and to unleash them in the city to 'purge' it once and for all.",
                    [2] = "The entrance to their inner sanctum has been opened for you. Be prepared for a HARD battle! Better gather some friends to assist you.",
                    [3] = "Report back to whichever principal you have chosen to help and you will receive Yalaharian Outfits.",
                    [4] = "You got the access to the reward room. Choose carefully which reward you pick as you can only take one item.",
                    [5] = "You have completed the Quest!"
                }
            },
            {
                name = "The Way to Yalahar",
                storageId = Storage.TheWayToYalahar.QuestLine,  -- 30856 (CORRECTED)
                startValue = 1,
                endValue = 3,
                description = {
                    [1] = "Wyrdin asked you to find the explorer's society's captain Maximilian in Liberty Bay and ask him for a passage to Yalahar. There visit Timothy of the explorer's society and get his research notes.",
                    [2] = "You have obtained Timothy's research notes. Return them to Wyrdin in the Edron academy.",
                    [3] = "You have delivered the research notes to Wyrdin and received 500 gold pieces as reward."
                }
            },
            {
                name = "Sea Routes around Yalahar",
                storageId = Storage.SearoutesAroundYalahar.TownsCounter,
                startValue = 1,
                endValue = 5,
                description = {
                    [1] = "Karith asks you to prove that 5 major cities are worth establishing ship routes to. Bring items from Ab'Dendriel, Darashia, Venore, Ankrahmun, Port Hope, Thais, Liberty Bay, and Carlin. Cities proven: 1/5",
                    [2] = "You have proven 2 cities are worth ship routes. Continue gathering items from the remaining cities. Cities proven: 2/5",
                    [3] = "You have proven 3 cities are worth ship routes. Continue gathering items from the remaining cities. Cities proven: 3/5",
                    [4] = "You have proven 4 cities are worth ship routes. Bring one more city's item to establish the routes! Cities proven: 4/5",
                    [5] = "You have successfully completed Sea Routes around Yalahar! Ship routes are now established to Ab'Dendriel, Darashia, Venore, Ankrahmun, Port Hope, Thais, Liberty Bay and Carlin."
                }
            }
        }
    })

    quest:register()
    print(">> Registered quest: In Service of Yalahar (12 missions)")
    return true
end

inServiceOfYalahar:register()
