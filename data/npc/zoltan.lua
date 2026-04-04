-- Zoltan - Converted from XML to Lua NpcType
-- Original XML: data/npc/Zoltan.xml
-- Original Script: data/npc/scripts/Zoltan.lua

local npcName = "Zoltan"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a zoltan")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(2000)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 130, lookHead = 95, lookBody = 94, lookLegs = 95, lookFeet = 76})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)


local HAT_QUEST_STORAGE = Storage.OutfitQuest.MageSummoner.AddonHatCloak
local WAND_ADDON_STORAGE = Storage.OutfitQuest.MageSummoner.AddonWand

-- MASTER SPELL CONFIGURATION
local spells = {
    -- Attack Spells
    {name = 'Divine Caldera',    price = 3000, level = 50, vocations = {3, 7}, type = 'attack'},
    {name = 'Eternal Winter',    price = 8000, level = 60, vocations = {2, 6}, type = 'attack', storageId = Storage.Spells.EternalWinter},
    {name = 'Fierce Berserk',    price = 7500, level = 90, vocations = {4, 8}, type = 'attack'},
    {name = 'Hell\'s Core',      price = 8000, level = 60, vocations = {1, 5}, type = 'attack', storageId = Storage.Spells.HellsCore},
    {name = 'Rage of the Skies', price = 6000, level = 55, vocations = {1, 5}, type = 'attack', storageId = Storage.Spells.RageOfTheSkies},
    {name = 'Ultimate Energy Strike', price = 15000, level = 100, vocations = {1, 5}, type = 'attack', storageId = Storage.Spells.UltimateEnergyStrike},
    {name = 'Ultimate Flame Strike',  price = 15000, level = 90, vocations = {1, 5}, type = 'attack', storageId = Storage.Spells.UltimateFlameStrike},
    {name = 'Ultimate Ice Strike',   price = 15000, level = 100, vocations = {2, 6}, type = 'attack', storageId = Storage.Spells.UltimateIceStrike},
    {name = 'Ultimate Terra Strike', price = 15000, level = 90, vocations = {2, 6}, type = 'attack', storageId = Storage.Spells.UltimateTerraStrike},
    {name = 'Wrath of Nature',   price = 6000, level = 55, vocations = {2, 6}, type = 'attack', storageId = Storage.Spells.WrathOfNature},

    -- Healing Spells
    {name = 'Mass Healing',      price = 2200, level = 36, vocations = {2, 6}, type = 'healing'},

    -- Support Spells (Instant/Buffs)
    {name = 'Blood Rage',             price = 8000, level = 60, vocations = {4, 8}, type = 'support'},
    {name = 'Protector',              price = 6000, level = 55, vocations = {4, 8}, type = 'support'},
    {name = 'Sharpshooter',           price = 8000, level = 60, vocations = {3, 7}, type = 'support', storageId = Storage.Spells.Sharpshooter},
    {name = 'Swift Foot',             price = 6000, level = 55, vocations = {3, 7}, type = 'support', storageId = Storage.Spells.SwiftFoot},

    -- Conjure Spells (Runes/Ammo)
    {name = 'Conjure Bolt',           price = 750,  level = 17, vocations = {3, 7}, type = 'conjure'},
    {name = 'Paralyze Rune',          price = 1900, level = 54, vocations = {3, 7}, type = 'conjure'},
    {name = 'Conjure Piercing Bolt',  price = 850,  level = 33, vocations = {3, 7}, type = 'conjure'},
    {name = 'Conjure Poisoned Arrow', price = 700,  level = 16, vocations = {3, 7}, type = 'conjure'},
    {name = 'Conjure Sniper Arrow',   price = 800,  level = 24, vocations = {3, 7}, type = 'conjure'},
    {name = 'Energy Bomb Rune',       price = 2300, level = 37, vocations = {1, 5}, type = 'conjure'}
}

-- ULTIMATE SPELL PAIRS
local ultimateSwaps = {
    ["Hell's Core"] = "Rage of the Skies",
    ["Rage of the Skies"] = "Hell's Core",
    ["Eternal Winter"] = "Wrath of Nature",
    ["Wrath of Nature"] = "Eternal Winter",
    ["Sharpshooter"] = "Swift Foot",
    ["Swift Foot"] = "Sharpshooter",
    ["Ultimate Ice Strike"] = "Ultimate Terra Strike",
    ["Ultimate Terra Strike"] = "Ultimate Ice Strike",
    ["Ultimate Flame Strike"] = "Ultimate Energy Strike",
    ["Ultimate Energy Strike"] = "Ultimate Flame Strike"
}

local function hasVocation(player, allowedList)
    local pid = player:getVocation():getId()
    for _, vid in ipairs(allowedList) do
        if pid == vid then return true end
    end
    return false
end

local function getSpellData(name)
    for _, spell in ipairs(spells) do
        if spell.name == name then return spell end
    end
    return nil
end

local pendingSpell = {}
-- RARITY GEM TRADE SHOP
local shopItems = {
    {id = 18413, buy = 1000000,  sell = 0, subType = 0, name = "reroll gem"},
    {id = 18419, buy = 150000,  sell = 0, subType = 0, name = "rare gem"},
    {id = 18414, buy = 500000,  sell = 0, subType = 0, name = "epic gem"},
    {id = 24115, buy = 1000000, sell = 0, subType = 0, name = "legendary coin"},
    {id = 18420, buy = 2000000, sell = 0, subType = 0, name = "mythic gem"},
}

local function confirmSpellPurchase(cid)
    local player = Player(cid)
    local data = pendingSpell[cid]
    if not data then return false end
    
    if player:getTotalMoney() < data.price then
        npcHandler:say('You don\'t have enough money. This spell costs ' .. data.price .. ' gold.', cid)
        pendingSpell[cid] = nil
        return true
    end
    
    player:removeTotalMoney(data.price)
    
    if data.unlearn then
        player:forgetSpell(data.unlearn)
        npcHandler:say('You have forgotten ' .. data.unlearn .. ' and learned ' .. data.spellName .. '.', cid)
    else
        npcHandler:say('You have learned ' .. data.spellName .. '.', cid)
    end
    
    player:learnSpell(data.spellName)
    player:getPosition():sendMagicEffect(CONST_ME_MAGIC_BLUE)
    
    if data.storageId then
        player:setStorageValue(data.storageId, 1)
    end
    
    pendingSpell[cid] = nil
    return true
end

local function getShopItem(itemId, subType, isBuying)
    for _, item in ipairs(shopItems) do
        if item.id == itemId then
            if isBuying and item.buy > 0 then
                return item
            elseif not isBuying and item.sell > 0 then
                return item
            end
        end
    end
    return nil
end

local function offerSpell(cid, spellName, price, level, vocations)
    local player = Player(cid)
    local spellData = getSpellData(spellName) 
    
    if player:hasLearnedSpell(spellName) then
        npcHandler:say('You already know this spell.', cid)
        return true
    end
    
    if player:getLevel() < level then
        npcHandler:say('You must be at least level ' .. level .. ' to learn this spell.', cid)
        return true
    end
    
    if not hasVocation(player, vocations) then
        npcHandler:say('This spell is not for your vocation.', cid)
        return true
    end
    
    local counterpart = ultimateSwaps[spellName]
    local isSwap = false
    local finalPrice = price
    
    if counterpart then
        if player:hasLearnedSpell(counterpart) then
            isSwap = true
            if spellData.storageId and player:getStorageValue(spellData.storageId) == 1 then
                finalPrice = 500
            else
                finalPrice = price
            end
        end
    end
    
    if player:getMoney() < finalPrice then
        npcHandler:say('You don\'t have enough money. This spell costs ' .. finalPrice .. ' gold.', cid)
        return true
    end
    
    pendingSpell[cid] = {
        spellName = spellName, 
        price = finalPrice, 
        unlearn = isSwap and counterpart or nil,
        storageId = spellData.storageId
    }

    if isSwap then
        if finalPrice == 500 then
            npcHandler:say('You already know ' .. counterpart .. '. Since you have mastered ' .. spellName .. ' before, I can switch it back for 500 gold. Do you want to proceed?', cid)
        else
            npcHandler:say('You already know ' .. counterpart .. '. To switch to ' .. spellName .. ', you must pay the full price of ' .. finalPrice .. ' gold. Do you want to proceed?', cid)
        end
    else
        npcHandler:say('Would you like to purchase the ' .. spellName .. ' spell for ' .. finalPrice .. ' gold?', cid)
    end
    
    npcHandler.topic[cid] = 999 
    return true
end

local function creatureSayCallback(cid, type, msg)
    if not npcHandler:isFocused(cid) then return false end

    local player = Player(cid)
    local m = msg:lower()

    -- Spell confirmation (topic 999)
    if npcHandler.topic[cid] == 999 then
        if m == 'yes' then
            confirmSpellPurchase(cid)
        elseif m == 'no' then
            npcHandler:say('Maybe another time then.', cid)
            pendingSpell[cid] = nil
        end
        npcHandler.topic[cid] = 0
        return true
    end


    if m == 'spells' then
        npcHandler:say("I can teach you {attack spells}, {healing spells}, {support spells} and {conjure spells}.", cid)
        return true
    end

    if m:find('attack') or m:find('healing') or m:find('support') or m:find('conjure') then
        local category = 'attack'
        if m:find('healing') then category = 'healing'
        elseif m:find('support') then category = 'support'
        elseif m:find('conjure') then category = 'conjure' end

        local available = {}
        for _, spell in ipairs(spells) do
            if spell.type == category and not player:hasLearnedSpell(spell.name) and hasVocation(player, spell.vocations) then
                table.insert(available, spell.name)
            end
        end
        
        if #available > 0 then
            local list = ""
            for i, name in ipairs(available) do
                if i == 1 then list = "'{" .. name .. "}'"
                elseif i == #available then list = list .. " and '{" .. name .. "}'"
                else list = list .. ", '{" .. name .. "}'" end
            end
            npcHandler:say("In this category I have " .. list .. ".", cid)
        else
            npcHandler:say("You have already learned all the " .. category .. " spells I can teach for your vocation.", cid)
        end
        return true
    end

    -- QUEST: FERUMBRAS HAT (PROOF)
    if m == 'proof' then
        local hasHat = player:getItemCount(5903) > 0
        local sexId = player:getSex()
        local alreadyHasAddon = player:hasOutfit(sexId == PLAYERSEX_FEMALE and 141 or 130, 2)

        if alreadyHasAddon then
            npcHandler:say('You have already proven yourself worthy and received the Ferumbras\' hat addon.', cid)
            return true
        elseif not hasHat then
            npcHandler:say('Ah, you seek to prove yourself against the mighty Ferumbras? Bring me his hat as proof. But first, you must complete the wand addon quest with Lynda.', cid)
            return true
        elseif hasHat then
            npcHandler:say('... I cannot believe my eyes. You retrieved this hat from Ferumbras\' remains? That is incredible. But first you need to prove yourself worthy by completing the wand addon quest with Lynda in Thais. What do you say?', cid)
            npcHandler.topic[cid] = 1
            return true
        end
    end

    if npcHandler.topic[cid] == 1 and m == 'yes' then
        if player:removeItem(5903, 1) then
            npcHandler:say('I bow to you, player, and hereby grant you the right to wear Ferumbras\' hat as accessory. Congratulations!', cid)
            player:addOutfitAddon(141, 2)
            player:addOutfitAddon(130, 2)
            player:getPosition():sendMagicEffect(CONST_ME_MAGIC_BLUE)
        else
            npcHandler:say('Sorry you don\'t have the Ferumbras\' hat.', cid)
        end
        npcHandler.topic[cid] = 0
        return true
    end

    -- QUEST: MYRA'S HANDOFF
    if m == 'myra' then
        local storage = player:getStorageValue(HAT_QUEST_STORAGE)
        if storage == 11 then
            npcHandler:say('Stop bothering me. I am a far too busy man to be constantly giving out awards.', cid)
        elseif storage == 10 then
            npcHandler:say({'Bah, I know. I received some sort of \'nomination\' from our outpost in Port Hope. ...', 'I hereby grant you the right to wear a special sign of honour. There you go.'}, cid)
            player:addOutfitAddon(138, 2)
            player:addOutfitAddon(133, 2)
            player:getPosition():sendMagicEffect(CONST_ME_MAGIC_BLUE)
            player:setStorageValue(HAT_QUEST_STORAGE, 11)
            player:setStorageValue(Storage.OutfitQuest.MageSummoner.MissionHatCloak, 0)
            player:setStorageValue(Storage.OutfitQuest.Ref, math.min(0, player:getStorageValue(Storage.OutfitQuest.Ref) - 1))
        else
            npcHandler:say('What the hell are you talking about?', cid)
        end
        return true
    end

    return true
end

npcHandler:setMessage(MESSAGE_GREET, 'Welcome |PLAYERNAME|, student of the arcane arts. I can teach you {spells} or you can browse my {trade} shop for rarity gems.')
npcHandler:setMessage(MESSAGE_FAREWELL, 'Good bye, and don\'t come back too soon.')
npcHandler:setMessage(MESSAGE_WALKAWAY, 'Good bye, and don\'t come back too soon.')

npcHandler:setCallback(CALLBACK_MESSAGE_DEFAULT, creatureSayCallback)
npcHandler:addModule(FocusModule:new(), npcHandler.keywordHandler)

-- Use full spell name as keyword to avoid partial matches (e.g., "fireball rune" vs "great fireball rune")
-- Sort spells by name length (longest first) to ensure longer names match before shorter substrings
local sortedSpells = {}
for _, spell in ipairs(spells) do
    table.insert(sortedSpells, spell)
end
table.sort(sortedSpells, function(a, b) return #a.name > #b.name end)

for _, spell in ipairs(sortedSpells) do
    keywordHandler:addKeyword({spell.name:lower()}, function(cid) return offerSpell(cid, spell.name, spell.price, spell.level, spell.vocations) end)
end


-- Trade window for rarity gems
local function openTradeWindow(cid, message, keywords, parameters, node)
    if not npcHandler:isFocused(cid) then return false end
    local player = Player(cid)
    if not player then return false end
    local npc = Npc(getNpcCid())
    local shopList = {}
    for _, item in ipairs(shopItems) do
        table.insert(shopList, {id = item.id, buy = item.buy, sell = item.sell, subType = item.subType or 0, name = item.name})
    end
    npc:openShopWindow(player, shopList, function() return true end, function() return true end)
    npcHandler:say('Browse my collection of rarity gems. Use them wisely!', cid)
    return true
end
keywordHandler:addKeyword({'trade'}, openTradeWindow, {npcHandler = npcHandler})

-- NpcType callbacks (MUST call setCurrentNpc first!)
npcType:eventType(NPCS_EVENT_APPEAR)
npcType:onAppear(function(npc, creature)
    setCurrentNpc(npc)
    npcHandler:onCreatureAppear(creature)
end)

npcType:eventType(NPCS_EVENT_DISAPPEAR)
npcType:onDisappear(function(npc, creature)
    setCurrentNpc(npc)
    npcHandler:onCreatureDisappear(creature)
end)

npcType:eventType(NPCS_EVENT_SAY)
npcType:onSay(function(npc, creature, type, message)
    setCurrentNpc(npc)
    npcHandler:onCreatureSay(creature, type, message)
end)

npcType:eventType(NPCS_EVENT_THINK)
npcType:onThink(function(npc, interval)
    setCurrentNpc(npc)
    npcHandler:onThink()
end)

npcType:eventType(NPCS_EVENT_CLOSECHANNEL)
npcType:onCloseChannel(function(npc, creature)
    setCurrentNpc(npc)
    npcHandler:onPlayerCloseChannel(creature)
end)

npcType:eventType(NPCS_EVENT_BUYITEM)
npcType:onBuyItem(function(npc, player, itemId, subType, amount, ignoreCap, inBackpacks)
    local shopItem = getShopItem(itemId, subType, true)
    if not shopItem or shopItem.buy <= 0 then return false end
    local totalCost = amount * shopItem.buy
    if player:getTotalMoney() < totalCost then
        player:sendCancelMessage("You don't have enough money.")
        return false
    end
    local itemSubType = shopItem.subType or 1
    local bought = doNpcSellItem(player:getId(), itemId, amount, itemSubType, ignoreCap, inBackpacks, ITEM_BACKPACK)
    if bought == 0 then
        player:sendCancelMessage("You do not have enough capacity.")
        return false
    end
    player:removeTotalMoney(bought * shopItem.buy)
    player:sendTextMessage(MESSAGE_INFO_DESCR, "Bought " .. bought .. "x " .. shopItem.name .. " for " .. (bought * shopItem.buy) .. " gold.")
    return true
end)

npcHandler:addModule(FocusModule:new())
npcType:register()
