-- The Explorer Society Quest
-- Converted from quests.xml to Lua
-- Quest NPCs: Berenice (Calassa), Lurik (Ice Music, Island of Dragons)
-- Storage keys defined in data/lib/core/storages.lua under Storage.ExplorerSociety

local explorerSociety = GlobalEvent("ExplorerSocietyQuest")

function explorerSociety.onStartup()
    local quest = Game.createQuest("The Explorer Society", {
        storageId = Storage.ExplorerSociety.QuestLine,  -- 30154
        storageValue = 1,
        missions = {
            {
                name = "Joining the Explorers",
                storageId = Storage.ExplorerSociety.JoiningtheExplorers,  -- 30152
                startValue = 1,
                endValue = 4,
                description = {
                    [1] = "The mission should be simple to fulfil. You have to seek out Uzgod in Kazordoon and get the pickaxe for us.",
                    [2] = "Get into Dwacatra and bring family brooch back to Uzgod.",
                    [3] = "Bring the pickaxe back to the Explorer Society representative.",
                    [4] = "You have joined the explorer society."
                }
            },
            {
                name = "The Ice Delivery",
                storageId = Storage.ExplorerSociety.TheIceDelivery,  -- 30162
                startValue = 1,
                endValue = 7,
                description = {
                    [1] = "Take this ice pick and use it on a block of ice in the caves beneath Folda. Get some ice and bring it here as fast as you can. If the ice melt away, report on your ice delivery mission anyway.",
                    [2] = "You have 10 minutes before the icicle defrosts. Run back to the Explorer Society representative!",
                    [3] = "Bring the ice back to the Explorer Society representative.",
                    [4] = "Bring the ice back to the Explorer Society representative.",
                    [5] = "Bring the ice back to the Explorer Society representative.",
                    [6] = "Bring the ice back to the Explorer Society representative.",
                    [7] = "Bring the ice back to the Explorer Society representative."
                }
            },
            {
                name = "The Butterfly Hunt",
                storageId = Storage.ExplorerSociety.TheButterflyHunt,  -- 30159
                startValue = 8,
                endValue = 16,
                description = {
                    [8] = "This preparation kit will allow you to collect a PURPLE butterfly you have killed. Just use it on the fresh corpse of a PURPLE butterfly.",
                    [9] = "Return the prepared butterfly to Explorer Society representative.",
                    [10] = "Ask for another butterfly hunt.",
                    [11] = "This preparation kit will allow you to collect a BLUE butterfly you have killed. Just use it on the fresh corpse of a BLUE butterfly.",
                    [12] = "Return the prepared butterfly to Explorer Society representative.",
                    [13] = "Ask for another butterfly hunt.",
                    [14] = "This preparation kit will allow you to collect a RED butterfly you have killed. Just use it on the fresh corpse of a RED butterfly.",
                    [15] = "Return the prepared butterfly to Explorer Society representative.",
                    [16] = "You completed the butterfly hunt!"
                }
            },
            {
                name = "The Plant Collection",
                storageId = Storage.ExplorerSociety.ThePlantCollection,  -- 30168
                startValue = 17,
                endValue = 26,
                description = {
                    [17] = "Take botanist's container. Use it on a jungle bells plant to collect a sample.",
                    [18] = "Report about your plant collection to Explorer Society representative.",
                    [19] = "Ask for plant collection when you are ready to continue.",
                    [20] = "Use botanist's container on a witches cauldron to collect a sample.",
                    [21] = "Report about your plant collection to Explorer Society representative.",
                    [22] = "Ask for plant collection when you are ready to continue.",
                    [23] = "Use this botanist's container on a giant jungle rose to obtain a sample.",
                    [24] = "Report about your plant collection to Explorer Society representative."
                }
            },
            {
                name = "The Lizard Urn",
                storageId = Storage.ExplorerSociety.TheLizardUrn,  -- 30165
                startValue = 27,
                endValue = 29,
                description = {
                    [27] = "In the south-east of Tiquanda is a small settlement of the lizard people. Beneath the newly constructed temple there, the lizards hide the urn. Acquire an ancient urn which is some sort of relic to the lizard people of Tiquanda.",
                    [28] = "Bring the Funeral Urn back to the Explorer Society."
                }
            },
            {
                name = "The Bonelord Secret",
                storageId = Storage.ExplorerSociety.TheBonelordSecret,  -- 30158
                startValue = 30,
                endValue = 32,
                description = {
                    [30] = "Travel to the city of Darashia and then head north-east for the pyramid. If any documents are left, you probably find them in the catacombs beneath.",
                    [31] = "Bring the Wrinkled Parchment back to the Explorer Society representative."
                }
            },
            {
                name = "The Orc Powder",
                storageId = Storage.ExplorerSociety.TheOrcPowder,  -- 30167
                startValue = 33,
                endValue = 35,
                description = {
                    [33] = "As far as we can tell, the orcs maintain some sort of training facility in some hill in the north-east of their city. There you should find lots of their war wolves and hopefully also some of the orcish powder.",
                    [34] = "Bring the Strange Powder to the Explorer Society representative to complete your mission."
                }
            },
            {
                name = "The Elven Poetry",
                storageId = Storage.ExplorerSociety.TheElvenPoetry,  -- 30161
                startValue = 36,
                endValue = 38,
                description = {
                    [36] = "This mission is easy but nonetheless vital. Travel Hellgate beneath Ab'Dendriel and get the book.",
                    [37] = "Bring back an elven poetry book to the Explorer Society representative."
                }
            },
            {
                name = "The Memory Stone",
                storageId = Storage.ExplorerSociety.TheMemoryStone,  -- 30166
                startValue = 39,
                endValue = 41,
                description = {
                    [39] = "In the ruins of north-western Edron you should be able to find a memory stone.",
                    [40] = "Bring back a memory stone to the Explorer Society representative."
                }
            },
            {
                name = "The Rune Writings",
                storageId = Storage.ExplorerSociety.TheRuneWritings,  -- 30169
                startValue = 42,
                endValue = 44,
                description = {
                    [42] = "Somewhere under the ape infested city of Banuta, one can find dungeons that were once inhabited by lizards. Look there for an atypical structure that would rather fit to Ankrahmun and its Ankrahmun Tombs. Copy the runes you will find on this",
                    [43] = "Report back to the Explorer Society representative."
                }
            },
            {
                name = "The Ectoplasm",
                storageId = Storage.ExplorerSociety.TheEctoplasm,  -- 30160
                startValue = 45,
                endValue = 47,
                description = {
                    [48] = "Take ectoplasm container and use it on a ghost that was recently slain.",
                    [49] = "Return back to the Explorer Society representative with the collected ectoplasm."
                }
            },
            {
                name = "The Spectral Dress",
                storageId = Storage.ExplorerSociety.TheSpectralDress,  -- 30170
                startValue = 48,
                endValue = 50,
                description = {
                    [51] = "The queen of the banshees lives in the so called Ghostlands, south west of Carlin. Try to get a spectral dress from her.",
                    [52] = "Report to the Explorer Society with the spectral dress."
                }
            },
            {
                name = "The Spectral Stone",
                storageId = Storage.ExplorerSociety.TheSpectralStone,  -- 30171
                startValue = 51,
                endValue = 55,
                description = {
                    [54] = "Please travel to our second base and ask them to mail us their latest research reports. Then return here and ask about new missions.",
                    [55] = "Tell our fellow explorer that the papers are in the mail already.",
                    [56] = "Take the spectral essence and use it on the strange carving in this building as well as on the corresponding tile in our second base.",
                    [57] = "Report back to the Explorer Society representative."
                }
            },
            {
                name = "The Astral Portals",
                storageId = Storage.ExplorerSociety.TheAstralPortals,  -- 30157
                startValue = 56,
                endValue = 56,
                description = {
                    [56] = "Both carvings are now charged and harmonised. You are able to travel in zero time from one base to the other, but you need to have an orichalcum pearl in your possession to use it as power source."
                }
            },
            {
                name = "The Island of Dragons",
                storageId = Storage.ExplorerSociety.TheIslandofDragons,  -- 30164
                startValue = 59,
                endValue = 60,
                description = {
                    [59] = "Travel to Okolnir and try to find a proof for the existence of dragon lords there in the old times. I think old Buddel might be able to bring you there.",
                    [60] = "Report back to Lurik with the dragon scale."
                }
            },
            {
                name = "The Ice Music",
                storageId = Storage.ExplorerSociety.TheIceMusic,  -- 30163
                startValue = 61,
                endValue = 63,
                description = {
                    [61] = "There is a cave on Hrodmir, north of the southernmost barbarian camp Krimhorn. In this cave, there are a waterfall and a lot of stalagmites. Take the resonance crystal and use it on the stalagmites in the cave to record the sound of the wind",
                    [62] = "Report back to Lurik.",
                    [63] = "Now you may use the Astral Bridge from Liberty Bay to Svargrond."
                }
            },
            {
                name = "The Undersea Kingdom",
                storageId = Storage.ExplorerSociety.CalassaQuest,  -- 30149
                startValue = 1,
                endValue = 3,
                description = {
                    [1] = "Captain Max will bring you to Calassa whenever you are ready. Please try to retrieve the missing logbook which must be in one of the sunken shipwrecks.",
                    [2] = "Report about your Calassa mission to Berenice in Liberty Bay.",
                    [3] = "You complete the task."
                }
            }
        }
    })

    quest:register()
    print(">> Registered quest: The Explorer Society (17 missions)")
    return true
end

explorerSociety:register()
