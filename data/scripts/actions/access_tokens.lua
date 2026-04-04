-- Access Token System (Revscript)
-- Players use items with action IDs to gain quest access
-- When used, tokens complete ALL missions for the respective faction

--[[
    ACTION ID REFERENCE TABLE
    ========================
    40001 - Efreet Djinn Access (Green Djinn faction)
    40002 - Marid Djinn Access (Blue Djinn faction)
    40003 - Travelling Trader Access (Rashid quest)
    40004 - In Service of Yalahar Access (All missions except final battle)
    40005 - The New Frontier Access (All missions except Mortal Combat)
    40006 - WoTe Access (Children of the Revolution + Wrath of the Emperor up to Mission 11)
    40007 - The Ape City Access (Complete quest - Shaman outfit)
    40008 - Deeper Banuta Shortcut Access
    40009 - Pits of Inferno Shortcut Access
    
    Add new action IDs here as you create them
]]

local accessTokens = {
    -- Efreet Djinn Access Token
    [40001] = {
        name = "Efreet Djinn Access",
        storage = 50001, -- Storage.PurchasedAccess.DjinnEfreet
        message = "You have gained access to trade with Efreet faction djinns!",
        missionsToComplete = {
            -- Start the Efreet quest line (this makes the quest appear in quest log)
            {storage = Storage.DjinnWar.EfreetFaction.Start, value = 1},
            -- Start and complete all Efreet missions (set to end value to mark as complete)
            {storage = Storage.DjinnWar.EfreetFaction.Mission01, value = 3}, -- Mission 1 complete
            {storage = Storage.DjinnWar.EfreetFaction.Mission02, value = 3}, -- Mission 2 complete
            {storage = Storage.DjinnWar.EfreetFaction.Mission03, value = 3}, -- Mission 3 complete (allows trading)
            -- Set faction choice in Factions quest
            {storage = Storage.DjinnWar.Faction.Efreet, value = 1}, -- Efreet faction chosen
            {storage = Storage.DjinnWar.Faction.Greeting, value = 0}, -- Greeting flag (0 = joined faction)
            {storage = Storage.Factions, value = 2}, -- Factions quest storage
            -- Door access
            {storage = Storage.DjinnWar.EfreetFaction.DoorToMaridTerritory, value = 1}
        }
    },
    -- Marid Djinn Access Token
    [40002] = {
        name = "Marid Djinn Access",
        storage = 50002, -- Storage.PurchasedAccess.DjinnMarid
        message = "You have gained access to trade with Marid faction djinns!",
        missionsToComplete = {
            -- Start the Marid quest line (this makes the quest appear in quest log)
            {storage = Storage.DjinnWar.MaridFaction.Start, value = 1},
            -- Start and complete all Marid missions (set to end value to mark as complete)
            {storage = Storage.DjinnWar.MaridFaction.Mission01, value = 2}, -- Mission 1 complete
            {storage = Storage.DjinnWar.MaridFaction.Mission02, value = 2}, -- Mission 2 complete
            {storage = Storage.DjinnWar.MaridFaction.RataMari, value = 2}, -- Rata'Mari quest complete
            {storage = Storage.DjinnWar.MaridFaction.Mission03, value = 3}, -- Mission 3 complete (allows trading)
            -- Set faction choice in Factions quest
            {storage = Storage.DjinnWar.Faction.Marid, value = 1}, -- Marid faction chosen
            {storage = Storage.DjinnWar.Faction.Greeting, value = 0}, -- Greeting flag (0 = joined faction)
            {storage = Storage.Factions, value = 2}, -- Factions quest storage
            -- Door access
            {storage = Storage.DjinnWar.MaridFaction.DoorToEfreetTerritory, value = 1}
        }
    },
    -- Travelling Trader Access Token
    [40003] = {
        name = "Travelling Trader Access",
        storage = 50003, -- Storage.PurchasedAccess.TravellingTrader
        message = "You have gained access to trade with Rashid!",
        missionsToComplete = {
            -- Start the quest (makes it appear in quest log)
            {storage = Storage.TheTravellingTraderQuest.Questline, value = 1}, -- 51201
            -- Complete all missions
            {storage = Storage.TheTravellingTraderQuest.Mission01, value = 2}, -- 51202 - Trophy mission
            {storage = Storage.TheTravellingTraderQuest.Mission02, value = 5}, -- 51203 - Delivery mission
            {storage = Storage.TheTravellingTraderQuest.Mission03, value = 3}, -- 51204 - Cheese mission
            {storage = Storage.TheTravellingTraderQuest.Mission04, value = 3}, -- 51205 - Vase mission
            {storage = Storage.TheTravellingTraderQuest.Mission05, value = 3}, -- 51206 - Crimson Sword mission
            {storage = Storage.TheTravellingTraderQuest.Mission06, value = 2}, -- 51207 - Goldfish mission
            {storage = Storage.TheTravellingTraderQuest.Mission07, value = 1}  -- 51208 - Declared as trader (allows trading)
        }
    },
    -- In Service of Yalahar Access Token (All missions except final battle)
    [40004] = {
        name = "In Service of Yalahar Access",
        storage = 50004, -- Storage.PurchasedAccess.InServiceOfYalahar
        message = "You have completed all In Service of Yalahar missions except the final battle!",
        missionsToComplete = {
            -- Start the quest (makes it appear in quest log)
            -- Wyrdin sets this to 1 when you accept the mission - this triggers the quest to appear
            {storage = Storage.TheWayToYalahar.QuestLine, value = 1}, -- 30856 (CORRECTED) - Set to 1 to make quest appear
            -- Set main In Service questline to value 50 (ready for final battle decision)
            {storage = Storage.InServiceofYalahar.Questline, value = 50}, -- 30224 - Main quest progress
            -- Complete missions 1-9 (setting to end values marks them as complete)
            {storage = Storage.InServiceofYalahar.Mission01, value = 6}, -- 30205 - Something Rotten (startValue=1, endValue=6)
            {storage = Storage.InServiceofYalahar.Mission02, value = 8}, -- 30206 - Watching the Watchmen (startValue=1, endValue=8)
            {storage = Storage.InServiceofYalahar.Mission03, value = 6}, -- 30207 - Death to the Deathbringer (startValue=1, endValue=6)
            {storage = Storage.InServiceofYalahar.Mission04, value = 6}, -- 30208 - Good to be Kingpin (startValue=1, endValue=6)
            {storage = Storage.InServiceofYalahar.Mission05, value = 8}, -- 30209 - Food or Fight (startValue=1, endValue=8)
            {storage = Storage.InServiceofYalahar.Mission06, value = 5}, -- 30210 - Frightening Fuel (startValue=1, endValue=5)
            {storage = Storage.InServiceofYalahar.Mission07, value = 5}, -- 30211 - A Fishy Mission (startValue=1, endValue=5)
            {storage = Storage.InServiceofYalahar.Mission08, value = 4}, -- 30212 - Dangerous Machinations (startValue=1, endValue=4)
            {storage = Storage.InServiceofYalahar.Mission09, value = 2}, -- 30213 - Decision (startValue=1, endValue=2)
            -- Mission 10 (Final Battle) - NOT set, player must start it themselves
            -- Door and gate access
            {storage = Storage.InServiceofYalahar.DoorToAzerus, value = 1}, -- 30196
            {storage = Storage.InServiceofYalahar.DoorToBog, value = 1}, -- 30197
            {storage = Storage.InServiceofYalahar.DoorToQuara, value = 1}, -- 30200
            -- Side tracking (good side by default)
            {storage = Storage.InServiceofYalahar.GoodSide, value = 4}, -- 30202
            -- Sea Routes around Yalahar (complete)
            {storage = Storage.SearoutesAroundYalahar.TownsCounter, value = 5}, -- All 5 cities (startValue=1, endValue=5)
            -- Additional door access
            {storage = 12272, value = 1}, -- Research notes door
            {storage = 12243, value = 1}, -- Door to Azerus
            {storage = 12278, value = 1}  -- Quara leaders door
        }
    },
    -- The New Frontier Access Token (All missions except Mortal Combat)
    [40005] = {
        name = "The New Frontier Access",
        storage = 50005, -- Storage.PurchasedAccess.TheNewFrontier
        message = "You have completed all The New Frontier missions except Mortal Combat!",
        missionsToComplete = {
            -- Set Questline to 25 - ready for Mortal Combat arena
            {storage = Storage.TheNewFrontier.Questline, value = 25}, -- 12130 - Ready for Mortal Combat
            -- Complete missions 1-8 (all except Mortal Combat)
            {storage = Storage.TheNewFrontier.Mission01, value = 3}, -- 12131 - New Land (endValue = 3)
            {storage = Storage.TheNewFrontier.Mission02, value = 6}, -- 12132 - From Kazordoon With Love (endValue = 6)
            {storage = Storage.TheNewFrontier.Mission03, value = 3}, -- 12133 - Strangers in the Night (endValue = 3)
            {storage = Storage.TheNewFrontier.Mission04, value = 2}, -- 12134 - The Mine Is Mine (endValue = 2)
            {storage = Storage.TheNewFrontier.Mission05, value = 7}, -- 12135 - Getting Things Busy (endValue = 7)
            {storage = Storage.TheNewFrontier.Mission06, value = 3}, -- 12136 - Days Of Doom (endValue = 3)
            {storage = Storage.TheNewFrontier.Mission07, value = 3}, -- 12137 - Messengers Of Peace (endValue = 3)
            {storage = Storage.TheNewFrontier.Mission08, value = 2}, -- 12138 - An Offer You Can't Refuse (endValue = 2)
            -- Mission 09 (Mortal Combat) - set to 1 (started but not completed)
            {storage = Storage.TheNewFrontier.Mission09, value = 1}, -- 12139 - Mortal Combat (NOT completed, player must do it)
            -- Mission 10 and Tome of Knowledge - NOT set, player gets these after completing Mission 09
        },
        -- Register creature event for Tirecz kill
        registerEvent = "NewFrontierTirecz"
    },
    -- Farmine Access Token (Children of the Revolution + Wrath of the Emperor up to Mission 11)
    [40006] = {
        name = "Farmine Access",
        storage = 50006, -- Storage.PurchasedAccess.Farmine
        message = "You have completed Children of the Revolution and Wrath of the Emperor up to Mission 11!",
        questlineInit = {
            -- Initialize questlines first to trigger quest log appearance
            {storage = Storage.ChildrenoftheRevolution.Questline, value = 1},
            {storage = Storage.WrathoftheEmperor.Questline, value = 1},
        },
        missionsToComplete = {
            -- Children of the Revolution - Complete ALL missions
            {storage = Storage.ChildrenoftheRevolution.Questline, value = 21}, -- Quest complete (21 = finished, ready for WotE)
            {storage = Storage.ChildrenoftheRevolution.Mission00, value = 2}, -- Prove Your Worzz! (complete)
            {storage = Storage.ChildrenoftheRevolution.Mission01, value = 3}, -- Mission 1: Corruption (complete)
            {storage = Storage.ChildrenoftheRevolution.Mission02, value = 5}, -- Mission 2: Imperial Zzecret Weaponzz (complete)
            {storage = Storage.ChildrenoftheRevolution.Mission03, value = 3}, -- Mission 3: Zee Killing Fieldzz (complete)
            {storage = Storage.ChildrenoftheRevolution.Mission04, value = 6}, -- Mission 4: Zze Way of Zztonezz (complete)
            {storage = Storage.ChildrenoftheRevolution.Mission05, value = 3}, -- Mission 5: Phantom Army (complete)
            
            -- Wrath of the Emperor - Complete missions 1-10, start mission 11
            {storage = Storage.WrathoftheEmperor.Questline, value = 31}, -- Questline at mission 11 start
            {storage = Storage.WrathoftheEmperor.Mission01, value = 3}, -- Catering the Lions Den (complete)
            {storage = Storage.WrathoftheEmperor.Mission02, value = 3}, -- First Contact (complete)
            {storage = Storage.WrathoftheEmperor.Mission03, value = 3}, -- The Keeper (complete)
            {storage = Storage.WrathoftheEmperor.Mission04, value = 3}, -- Sacrament of the Snake (complete)
            {storage = Storage.WrathoftheEmperor.Mission05, value = 3}, -- New in Town (complete)
            {storage = Storage.WrathoftheEmperor.Mission06, value = 4}, -- The Office Job (complete)
            {storage = Storage.WrathoftheEmperor.Mission07, value = 6}, -- A Noble Cause (complete)
            {storage = Storage.WrathoftheEmperor.Mission08, value = 2}, -- Uninvited Guests (complete)
            {storage = Storage.WrathoftheEmperor.Mission09, value = 2}, -- The Sleeping Dragon (complete)
            {storage = Storage.WrathoftheEmperor.Mission10, value = 6}, -- A Message of Freedom (complete)
            {storage = Storage.WrathoftheEmperor.Mission11, value = 1}, -- Payback Time (STARTED, not complete)
            {storage = Storage.WrathoftheEmperor.BossStatus, value = 1}, -- Boss status for mission 10
            -- Mission 12 - NOT set, player must complete Mission 11 first
        }
    },
    -- The Ape City Access Token (Complete quest - Shaman outfit access)
    [40007] = {
        name = "The Ape City Access",
        storage = 50007, -- Storage.PurchasedAccess.ApeCity
        message = "You have completed The Ape City quest and received the Shaman outfit!",
        missionsToComplete = {
            -- Set quest progress to final value (18 = quest complete)
            -- This also triggers the quest to appear since storageId == Questline
            {storage = Storage.TheApeCity.Questline, value = 18},  -- 30285
            -- Mark shaman outfit as received
            {storage = Storage.TheApeCity.ShamanOutfit, value = 1}
        },
        outfits = {
            {male = 154, female = 158}  -- Shaman outfit
        }
    },
    -- Deeper Banuta Shortcut Access Token
    [40008] = {
        name = "Deeper Banuta Shortcut Access",
        storage = 50008, -- Storage.PurchasedAccess.DeeperBanuta
        message = "You have gained access to the Deeper Banuta shortcut!",
        missionsToComplete = {
            {storage = 30276, value = 1}  -- DeeperBanutaShortcut storage
        }
    },
    -- Pits of Inferno Shortcut Access Token
    [40009] = {
        name = "Pits of Inferno Shortcut Access",
        storage = 50009, -- Storage.PurchasedAccess.PitsOfInferno
        message = "You have gained access to the Pits of Inferno shortcut!",
        missionsToComplete = {
            {storage = Storage.PitsOfInferno.ShortcutHub, value = 1}  -- 30043
        }
    },
}

local accessToken = Action()

function accessToken.onUse(player, item, fromPosition, target, toPosition, isHotkey)
    local actionId = item:getActionId()
    local tokenData = accessTokens[actionId]
    
    if not tokenData then
        return false
    end
    
    -- Check if already purchased
    if player:getStorageValue(tokenData.storage) == 1 then
        player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "You already have this access!")
        return true
    end
    
    -- Grant purchased access storage
    player:setStorageValue(tokenData.storage, 1)
    
    -- Complete ALL missions for this faction
    if tokenData.missionsToComplete then
        -- Check if this is a multi-quest token (has questlineInit)
        if tokenData.questlineInit then
            -- First, initialize the questlines to trigger quest log appearance
            for _, init in ipairs(tokenData.questlineInit) do
                player:setStorageValue(init.storage, init.value)
            end
            
            -- Then set all mission values with a small delay to ensure quest log updates
            addEvent(function(playerId)
                local p = Player(playerId)
                if p then
                    for _, mission in ipairs(tokenData.missionsToComplete) do
                        p:setStorageValue(mission.storage, mission.value)
                    end
                    p:sendTextMessage(MESSAGE_INFO_DESCR, "Quest progress updated! Relog to see updated quest log.")
                end
            end, 100, player:getId())
        else
            -- Standard token without delay
            for _, mission in ipairs(tokenData.missionsToComplete) do
                player:setStorageValue(mission.storage, mission.value)
            end
        end
    end
    
    -- Register creature event if specified
    if tokenData.registerEvent then
        player:registerEvent(tokenData.registerEvent)
    end
    
    -- Grant outfits if specified
    if tokenData.outfits then
        for _, outfit in ipairs(tokenData.outfits) do
            if player:getSex() == PLAYERSEX_FEMALE then
                player:addOutfit(outfit.female)
            else
                player:addOutfit(outfit.male)
            end
        end
    end
    
    -- Send success message
    player:sendTextMessage(MESSAGE_EVENT_ADVANCE, tokenData.message)
    
    item:getPosition():sendMagicEffect(CONST_ME_MAGIC_BLUE)
    item:remove(1) -- Consume the token
    
    return true
end

-- Register action IDs
for actionId, _ in pairs(accessTokens) do
    accessToken:aid(actionId)
end

accessToken:register()

