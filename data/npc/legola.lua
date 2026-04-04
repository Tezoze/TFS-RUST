-- Legola - Converted from XML to Lua NpcType
-- Original XML: data/npc/Legola.xml
-- Original Script: data/npc/scripts/Legola.lua

local npcName = "Legola"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a legola")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(2000)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 137, lookHead = 72, lookBody = 68, lookLegs = 68, lookFeet = 95})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)


local voices = { {text = 'Teaching paladin spells! Just come to me!'} }
npcHandler:addModule(VoiceModule:new(voices))

-- MASTER SPELL CONFIGURATION
-- Vocations: {3} = Paladin, {7} = Royal Paladin
local spells = {
    -- HEALING SPELLS
    {name = 'Cure Poison',          price = 150,   level = 10,  vocations = {3, 7}, type = 'healing'},
    {name = 'Divine Healing',       price = 3000,  level = 35,  vocations = {3, 7}, type = 'healing'},
    {name = 'Intense Healing',      price = 350,   level = 20,  vocations = {3, 7}, type = 'healing'},
    {name = 'Light Healing',        price = 0,     level = 8,   vocations = {3, 7}, type = 'healing'},

    -- SUPPORT SPELLS (Utility/Instant)
    {name = 'Cancel Invisibility',  price = 1600,  level = 26,  vocations = {3, 7}, type = 'support'},
    {name = 'Find Person',          price = 80,    level = 8,   vocations = {3, 7}, type = 'support'},
    {name = 'Great Light',          price = 500,   level = 13,  vocations = {3, 7}, type = 'support'},
    {name = 'Light',                price = 0,     level = 8,   vocations = {3, 7}, type = 'support'},

    -- CONJURE SPELLS (Ammo and Runes)
    {name = 'Conjure Arrow',        price = 450,   level = 13,  vocations = {3, 7}, type = 'conjure'},
    {name = 'Conjure Bolt',         price = 750,   level = 17,  vocations = {3, 7}, type = 'conjure'},
    {name = 'Conjure Explosive Arrow', price = 1000, level = 25, vocations = {3, 7}, type = 'conjure'},
    {name = 'Conjure Piercing Bolt',price = 850,   level = 33,  vocations = {3, 7}, type = 'conjure'},
    {name = 'Conjure Poisoned Arrow', price = 700, level = 16,  vocations = {3, 7}, type = 'conjure'},
    {name = 'Conjure Sniper Arrow', price = 800,   level = 24,  vocations = {3, 7}, type = 'conjure'},
    {name = 'Destroy Field Rune',   price = 700,   level = 17,  vocations = {3, 7}, type = 'conjure'}
}

-- Helper function to check vocation
local function hasVocation(player, allowedList)
    local pid = player:getVocation():getId()
    for _, vid in ipairs(allowedList) do
        if pid == vid then return true end
    end
    return false
end

-- Helper to check if player is a Paladin
local function isPaladin(player)
    local pid = player:getVocation():getId()
    return pid == 3 or pid == 7
end

-- Custom spell teaching with confirmation
local pendingSpell = {}

local function confirmSpellPurchase(cid)
    local player = Player(cid)
    local spellData = pendingSpell[cid]
    if not spellData then return false end
    
    if player:getTotalMoney() < spellData.price then
        npcHandler:say('You don\'t have enough money. This spell costs ' .. spellData.price .. ' gold.', cid)
        pendingSpell[cid] = nil
        return true
    end
    
    player:removeTotalMoney(spellData.price)
    player:learnSpell(spellData.name)
    
    if spellData.price > 0 then
        npcHandler:say('You have learned ' .. spellData.name .. ' for ' .. spellData.price .. ' gold!', cid)
    else
        npcHandler:say('You have learned ' .. spellData.name .. '!', cid)
    end
    
    player:getPosition():sendMagicEffect(CONST_ME_MAGIC_BLUE)
    pendingSpell[cid] = nil
    return true
end

local function offerSpell(cid, spellName, price, level, vocations)
    local player = Player(cid)
    if player:hasLearnedSpell(spellName) then
        npcHandler:say('You already know this spell.', cid)
        return true
    elseif player:getLevel() < level then
        npcHandler:say('You must be at least level ' .. level .. ' to learn this spell.', cid)
        return true
    elseif not hasVocation(player, vocations) then
        npcHandler:say('This spell is not for your vocation.', cid)
        return true
    elseif player:getTotalMoney() < price then
        npcHandler:say('You don\'t have enough money. This spell costs ' .. price .. ' gold.', cid)
        return true
    end
    
    pendingSpell[cid] = {name = spellName, price = price}
    npcHandler:say('Would you like to purchase the ' .. spellName .. ' spell for ' .. price .. ' gold?', cid)
    npcHandler.topic[cid] = 999 
    return true
end

local function creatureSayCallback(cid, type, msg)
    if not npcHandler:isFocused(cid) then return false end

    local player = Player(cid)
    local m = msg:lower()

    if npcHandler.topic[cid] == 999 then
        if m == 'yes' then confirmSpellPurchase(cid)
        elseif m == 'no' then npcHandler:say('Maybe another time then.', cid) pendingSpell[cid] = nil end
        npcHandler.topic[cid] = 0
        return true
    end

    -- SNIPER GLOVES LOGIC
    if m:find("sniper gloves") then
        local hasGloves = player:getItemCount(5875) > 0
        if not hasGloves then
            npcHandler:say('We are always looking for sniper gloves. If you find a pair, bring them here.', cid)
            return true
        end

        local alreadyHasAddon = player:hasOutfit(player:getSex() == PLAYERSEX_FEMALE and 137 or 129, 2)
        local isPremium = player:isPremium()

        if not isPremium or alreadyHasAddon then
            npcHandler:say('Would you like to sell them to me for 2000 gold pieces?', cid)
            npcHandler.topic[cid] = 10 
        else
            npcHandler:say('I will grant you the right to wear the sniper gloves accessory for them. How about it?', cid)
            npcHandler.topic[cid] = 11 
        end
        return true
    end

    -- YES/NO RESPONSES
    if m == "yes" then
        if npcHandler.topic[cid] == 10 then -- Sell
            if player:removeItem(5875, 1) then
                player:addMoney(2000)
                npcHandler:say('Alright! Here is your money.', cid)
            end
            npcHandler.topic[cid] = 0
        elseif npcHandler.topic[cid] == 11 then -- Trade
            if player:removeItem(5875, 1) then
                player:setStorageValue(Storage.OutfitQuest.Hunter.AddonGlove, 1)
                player:addOutfitAddon(129, 2)
                player:addOutfitAddon(137, 1)
                player:getPosition():sendMagicEffect(CONST_ME_MAGIC_BLUE)
                npcHandler:say('Great! Wear them proudly!', cid)
            end
            npcHandler.topic[cid] = 0
        end
        return true
    end

    -- SPELL TEACHING
    if m == 'spells' then
        if not isPaladin(player) then npcHandler:say("Sorry, I only teach paladins.", cid) return true end
        npcHandler:say('I can teach you {healing spells}, {support spells} and {conjure spells}.', cid)
        return true
    end

    if m:find('healing') or m:find('support') or m:find('conjure') then
        if not isPaladin(player) then npcHandler:say("Sorry, I only teach paladins.", cid) return true end
        local category = m:find('healing') and 'healing' or m:find('support') and 'support' or m:find('conjure') and 'conjure' or 'healing'
        local available = {}
        for _, spell in ipairs(spells) do
            if spell.type == category and not player:hasLearnedSpell(spell.name) and hasVocation(player, spell.vocations) then
                table.insert(available, spell.name)
            end
        end
        if #available > 0 then
            npcHandler:say("In this category I have " .. table.concat(available, ", ") .. ".", cid)
        else
            npcHandler:say("You already know all the " .. category .. " spells I can teach.", cid)
        end
        return true
    end

    return true
end

npcHandler:setMessage(MESSAGE_GREET, "Greetings, |PLAYERNAME|. I can teach you the ways of the paladin.")
npcHandler:setMessage(MESSAGE_FAREWELL, "Farewell, |PLAYERNAME|.")
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


-- Shop items (from XML parameters)
local shopItems = {
    {id = 12399, buy = 0, sell = 30, subType = 0, name = "girlish hair decoration"},
    {id = 12425, buy = 0, sell = 80, subType = 0, name = "hunters quiver"},
    {id = 12400, buy = 0, sell = 60, subType = 0, name = "protective charm"},
}

-- Helper function to find shop item by id and subType (for fluid containers)
local function getShopItem(itemId, subType, isBuying)
    local itemType = ItemType(itemId)
    if itemType:isFluidContainer() then
        for _, item in ipairs(shopItems) do
            if item.id == itemId and item.subType == subType then
                return item
            end
        end
    end
    -- For non-fluid items, find the entry that matches the operation
    for _, item in ipairs(shopItems) do
        if item.id == itemId then
            if isBuying and item.buy > 0 then
                return item
            elseif not isBuying and item.sell > 0 then
                return item
            end
        end
    end
    -- Fallback to first match
    for _, item in ipairs(shopItems) do
        if item.id == itemId then
            return item
        end
    end
    return nil
end

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
    npcHandler:say('Take all the time you need to browse my wares.', cid)
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

npcType:eventType(NPCS_EVENT_SELLITEM)
npcType:onSellItem(function(npc, player, itemId, subType, amount, ignoreEquipped)
    local shopItem = getShopItem(itemId, subType, false)
    if not shopItem or shopItem.sell <= 0 then return false end
    local totalPrice = amount * shopItem.sell
    local itemName = shopItem.name or ItemType(itemId):getName()
    
    local itemSubType = -1
    if ItemType(itemId):isFluidContainer() then
        itemSubType = subType
    end
    
    if doPlayerSellItem(player:getId(), itemId, amount, totalPrice, itemSubType, ignoreEquipped) then
        player:sendTextMessage(MESSAGE_INFO_DESCR, "Sold " .. amount .. "x " .. shopItem.name .. " for " .. (amount * shopItem.sell) .. " gold.")
        return true
    end
    player:sendCancelMessage("You do not have this object.")
    return false
end)

npcHandler:addModule(FocusModule:new())
npcType:register()
