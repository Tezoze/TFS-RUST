function onLogin(player)
    local serverName = configManager.getString(configKeys.SERVER_NAME)
    local loginStr = "Welcome to " .. serverName .. "!"
    if player:getLastLoginSaved() <= 0 then
        loginStr = loginStr .. " Please choose your outfit."
        player:sendOutfitWindow()
    else
        if loginStr ~= "" then
            player:sendTextMessage(MESSAGE_STATUS_DEFAULT, loginStr)
        end

        loginStr = string.format("Your last visit in %s: %s.", serverName, os.date("%d %b %Y %X", player:getLastLoginSaved()))
    end
    player:sendTextMessage(MESSAGE_STATUS_DEFAULT, loginStr)

    -- Promotion Fix
    local vocation = player:getVocation()
    local promotion = vocation:getPromotion()
    local value = player:getStorageValue(PlayerStorageKeys.promotion)
    if value == 1 and promotion then
        player:setVocation(promotion)
    end

    -- Events
    player:registerEvent("PlayerDeath")
    player:registerEvent("DropLoot")
    player:registerEvent("ExtendedOpcode")

    -- Re-register quest events based on current quest state
    local questline = player:getStorageValue(Storage.WrathoftheEmperor.Questline) or 0
    if questline >= 23 then
        player:registerEvent("WotELizardKill")
    end

    -- Wrath of the Emperor boss events (Mission 10)
    local mission10 = player:getStorageValue(Storage.WrathoftheEmperor.Mission10) or 0
    if mission10 >= 1 then
        player:registerEvent("WotEBosses")
    end

    -- Wrath of the Emperor Zalamon events (Mission 11)
    local mission11 = player:getStorageValue(Storage.WrathoftheEmperor.Mission11) or 0
    if mission11 >= 1 then
        player:registerEvent("WotEZalamon")
    end

    -- In Service of Yalahar quest events
    local yalaharQuestline = player:getStorageValue(Storage.InServiceofYalahar.Questline) or 0
    if yalaharQuestline >= 20 and yalaharQuestline < 22 then
        player:registerEvent("ServiceOfYalaharDiseasedTrio")
    end
    if yalaharQuestline >= 40 and yalaharQuestline < 43 then
        player:registerEvent("ServiceOfYalaharQuaraLeaders")
    end
    if yalaharQuestline >= 51 then
        player:registerEvent("ServiceOfYalaharAzerus")
    end

    -- The New Frontier quest events
    local newFrontierQuestline = player:getStorageValue(Storage.TheNewFrontier.Questline) or 0
    if newFrontierQuestline >= 12 and newFrontierQuestline < 14 then
        player:registerEvent("NewFrontierShardOfCorruption")
    end
    -- FIX: This range (25-27) correctly covers the Tirecz fight.
    if newFrontierQuestline >= 25 and newFrontierQuestline < 27 then
        player:registerEvent("NewFrontierTirecz")
    end

    -- Killing in the Name of quest events
    -- This logic is correct: It enables the tracking for anyone who has joined.
    local pawAndFurJoined = player:getStorageValue(Storage.KillingInTheNameOf.Join)
    if pawAndFurJoined >= 0 then
        -- NOTE: Ensure these names exist in creaturescripts.xml. 
        -- If you replaced your script with mine, verify which name you used.
        player:registerEvent("KillingInTheNameOfKills")
        player:registerEvent("KillingInTheNameOfKillss")
        player:registerEvent("KillingInTheNameOfKillsss")
        player:registerEvent("KillingInTheNameOfLogout")
    end

    dofile('data/lib/core/storages.lua')

    -- Gesior Shop System - Process pending gifts
    dofile('data/creaturescripts/scripts/gesior_shop_gifts.lua')
    processGesiorShopGifts(player)

    -- The Inquisition quest events
    local inquisitionQuestline = player:getStorageValue(12160) or 0 -- Storage.TheInquisition.Questline
    local inquisitionMission06 = player:getStorageValue(12166) or 0 -- Storage.TheInquisition.Mission06
    local inquisitionMission07 = player:getStorageValue(12167) or 0 -- Storage.TheInquisition.Mission07

    -- Inquisition Ungreez (Mission 6)
    if inquisitionMission06 >= 1 then
        player:registerEvent("InquisitionUngreez")
    end

    -- Inquisition Bosses (Mission 7)
    if inquisitionMission07 >= 1 then
        player:registerEvent("InquisitionBosses")
    end

    -- An Uneasy Alliance quest events (Mission 1)
    local uneasyAlliance = player:getStorageValue(Storage.AnUneasyAlliance) or 0
    if uneasyAlliance == 1 then
        player:registerEvent("UneasyAllianceRenegadeOrc")
    end

    -- Svargrond Arena quest events
    local arenaStage = player:getStorageValue(Storage.SvargrondArena.Arena) or 0
    if arenaStage >= 1 then
        player:registerEvent("SvargrondArenaKill")
    end

    -- Secret Service quest events
    local avinMission04 = player:getStorageValue(Storage.secretService.AVINMission04) or 0
    if avinMission04 == 1 then
        player:registerEvent("SecretServiceBlackKnight")
    end

    -- Activate Custom Item Attributes
    for i = 1,10 do -- CONST_SLOT_FIRST,CONST_SLOT_LAST
        local item = player:getSlotItem(i)
        if item then
            itemAttributes(player, item, i, true)
        end
    end

    -- If player logged with more 'current health' than their db 'max health' due to an item attribute
    local query = db.storeQuery("SELECT `health`,`mana` FROM players where `id`="..player:getGuid())
    if query then
        local health = tonumber(result.getDataString(query, 'health'))
        local mana = tonumber(result.getDataString(query, 'mana'))
        local playerHealth = player:getHealth()
        local playerMana = player:getMana()
        if playerHealth < health then
            player:addHealth(health - playerHealth)
        end
        if playerMana < mana then
            player:addMana(mana - playerMana)
        end
        result.free(query)
    end

    player:registerEvent("rollHealth")
    player:registerEvent("rollMana")
    
    -- Task System: Elite Spawn Check (Bigger and Badder)
    player:registerEvent("EliteSpawnCheck")
    player:registerEvent("EliteMonsterDamage")

    return true
end