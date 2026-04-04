-- Dario - Converted from XML to Lua NpcType
-- Original XML: data/npc/Dario.xml
-- Original Script: data/npc/scripts/Dario.lua

local npcName = "Dario"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a dario")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(2000)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 159, lookHead = 3, lookBody = 58, lookLegs = 41, lookFeet = 115})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)


-- Voice Module
local voices = {
    { text = 'Increase your knowledge of spells here, young paladin.' },
    { text = 'Need ammunition, bows or crossbows? Have a look at my wares.' }
}
npcHandler:addModule(VoiceModule:new(voices))

-- Custom Focus Module (Elven Greetings)
local focusModule = FocusModule:new()
focusModule:addGreetMessage({'hi', 'hello', 'ashari'})
focusModule:addFarewellMessage({'bye', 'farewell', 'asgha thrazi'})
npcHandler:addModule(focusModule)

-- MASTER SPELL CONFIGURATION
-- Vocations: {3} = Paladin, {7} = Royal Paladin
local spells = {
    -- ATTACK SPELLS
    {name = 'Divine Caldera',       price = 3000,  level = 50,  vocations = {3, 7}, type = 'attack'},
    {name = 'Divine Missile',       price = 1800,  level = 40,  vocations = {3, 7}, type = 'attack'},
    {name = 'Ethereal Spear',       price = 1100,  level = 23,  vocations = {3, 7}, type = 'attack'},
    {name = 'Holy Flash',       price = 7500,  level = 70,  vocations = {3, 7}, type = 'attack'},
    {name = 'Strong Ethereal Spear',       price = 10000,  level = 90,  vocations = {3, 7}, type = 'attack'},

    -- HEALING SPELLS
    {name = 'Cure Poison',          price = 150,   level = 10,  vocations = {3, 7}, type = 'healing'},
    {name = 'Divine Healing',       price = 3000,  level = 35,  vocations = {3, 7}, type = 'healing'},
    {name = 'Intense Healing',      price = 350,   level = 20,  vocations = {3, 7}, type = 'healing'},
    {name = 'Light Healing',        price = 0,     level = 8,   vocations = {3, 7}, type = 'healing'},
    {name = 'Recovery',             price = 4000,  level = 50,  vocations = {3, 7}, type = 'healing'},
    {name = 'Intense Recovery',     price = 10000, level = 100, vocations = {3, 7}, type = 'healing'},
    {name = 'Salvation',     price = 8000, level = 60, vocations = {3, 7}, type = 'healing'},
    {name = 'Cure Curse',     price = 6000, level = 80, vocations = {3, 7}, type = 'healing'},

    -- SUPPORT SPELLS (Instant Utility)
    {name = 'Cancel Invisibility',  price = 1600,  level = 26,  vocations = {3, 7}, type = 'support'},
    {name = 'Find Person',          price = 80,    level = 8,   vocations = {3, 7}, type = 'support'},
    {name = 'Great Light',          price = 500,   level = 13,  vocations = {3, 7}, type = 'support'},
    {name = 'Haste',                price = 600,   level = 14,  vocations = {3, 7}, type = 'support'},
    {name = 'Levitate',             price = 500,   level = 12,  vocations = {3, 7}, type = 'support'},
    {name = 'Light',                price = 0,     level = 8,   vocations = {3, 7}, type = 'support'},
    {name = 'Magic Rope',           price = 200,   level = 9,   vocations = {3, 7}, type = 'support'},

    -- CONJURE SPELLS (Ammo and Runes)
    {name = 'Conjure Arrow',        price = 450,   level = 13,  vocations = {3, 7}, type = 'conjure'},
    {name = 'Conjure Bolt',         price = 750,   level = 17,  vocations = {3, 7}, type = 'conjure'},
    {name = 'Conjure Explosive Arrow', price = 1000, level = 25, vocations = {3, 7}, type = 'conjure'},
    {name = 'Conjure Piercing Bolt',price = 850,   level = 33,  vocations = {3, 7}, type = 'conjure'},
    {name = 'Conjure Poisoned Arrow', price = 700, level = 16,  vocations = {3, 7}, type = 'conjure'},
    {name = 'Conjure Sniper Arrow', price = 800,   level = 24,  vocations = {3, 7}, type = 'conjure'},
    {name = 'Destroy Field Rune',   price = 700,   level = 17,  vocations = {3, 7}, type = 'conjure'},
    {name = 'Disintegrate Rune',    price = 900,   level = 21,  vocations = {3, 7}, type = 'conjure'},
    {name = 'Enchant Spear',        price = 2000,  level = 45,  vocations = {3, 7}, type = 'conjure'},
    {name = 'Holy Missile Rune',    price = 1600,  level = 27,  vocations = {3, 7}, type = 'conjure'}
}

local function hasVocation(player, allowedList)
    local pid = player:getVocation():getId()
    for _, vid in ipairs(allowedList) do
        if pid == vid then return true end
    end
    return false
end

local function isPaladin(player)
    local pid = player:getVocation():getId()
    return pid == 3 or pid == 7
end

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
    
    npcHandler:say('You have learned ' .. spellData.name .. (spellData.price > 0 and (' for ' .. spellData.price .. ' gold!') or '!'), cid)
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

    if m == 'spells' then
        if not isPaladin(player) then npcHandler:say("Sorry, I only teach paladins.", cid) return true end
        npcHandler:say('I can teach you {attack spells}, {healing spells}, {support spells} and {conjure spells}.', cid)
        return true
    end

    if m:find('attack') or m:find('healing') or m:find('support') or m:find('conjure') then
        if not isPaladin(player) then npcHandler:say("Sorry, I only teach paladins.", cid) return true end
        
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

    return true
end

npcHandler:setMessage(MESSAGE_GREET, "Ashari, |PLAYERNAME|. If you're a distance fighter, you might want to have a look at my wares and spells.")
npcHandler:setMessage(MESSAGE_FAREWELL, "Asha Thrazi, |PLAYERNAME|.")
npcHandler:setCallback(CALLBACK_MESSAGE_DEFAULT, creatureSayCallback)

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
    {id = 2456, buy = 0, sell = 100, subType = 0, name = "bow"},
    {id = 2455, buy = 0, sell = 120, subType = 0, name = "crossbow"},
    {id = 2389, buy = 0, sell = 3, subType = 0, name = "spear"},
    {id = 2544, buy = 3, sell = 0, subType = 0, name = "arrow"},
    {id = 1294, buy = 15, sell = 0, subType = 0, name = "small stone"},
    {id = 2543, buy = 4, sell = 0, subType = 0, name = "bolt"},
    {id = 18304, buy = 20, sell = 0, subType = 0, name = "crystalline arrow"},
    {id = 18435, buy = 20, sell = 0, subType = 0, name = "prismatic bolt"},
    {id = 2456, buy = 400, sell = 0, subType = 0, name = "bow"},
    {id = 2455, buy = 500, sell = 0, subType = 0, name = "crossbow"},
    {id = 7850, buy = 5, sell = 0, subType = 0, name = "earth arrow"},
    {id = 7840, buy = 5, sell = 0, subType = 0, name = "flaming arrow"},
    {id = 7838, buy = 5, sell = 0, subType = 0, name = "flash arrow"},
    {id = 7365, buy = 7, sell = 0, subType = 0, name = "onyx arrow"},
    {id = 7363, buy = 5, sell = 0, subType = 0, name = "piercing bolt"},
    {id = 2547, buy = 7, sell = 0, subType = 0, name = "power bolt"},
    {id = 7378, buy = 15, sell = 0, subType = 0, name = "royal spear"},
    {id = 7839, buy = 5, sell = 0, subType = 0, name = "shiver arrow"},
    {id = 7364, buy = 5, sell = 0, subType = 0, name = "sniper arrow"},
    {id = 2389, buy = 9, sell = 0, subType = 0, name = "spear"},
    {id = 2399, buy = 42, sell = 0, subType = 0, name = "throwing star"},
    {id = 28413, buy = 130, sell = 0, subType = 0, name = "diamond arrow"},
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

npcType:eventType(NPCS_EVENT_BUYITEM)
npcType:onBuyItem(function(npc, player, itemId, subType, amount, ignoreCap, inBackpacks)
    local shopItem = getShopItem(itemId, subType, true)
    if not shopItem or shopItem.buy <= 0 then return false end
    
    local totalCost = amount * shopItem.buy
    if player:getTotalMoney() < totalCost then
        player:sendCancelMessage("You don't have enough money.")
        return false
    end
    
    local bought = doNpcSellItem(player:getId(), itemId, amount, shopItem.subType or 1, ignoreCap, inBackpacks, ITEM_BACKPACK)
    if bought == 0 then
        player:sendCancelMessage("You do not have enough capacity.")
        return false
    end
    
    player:removeTotalMoney(bought * shopItem.buy)
    player:sendTextMessage(MESSAGE_INFO_DESCR, "Bought " .. bought .. "x " .. shopItem.name .. " for " .. (bought * shopItem.buy) .. " gold.")
    return true
end)

npcType:eventType(NPCS_EVENT_SELLITEM)
npcType:onSellItem(function(npc, player, itemId, subType, amount, ignoreEquipped)
    local shopItem = getShopItem(itemId, subType, false)
    if not shopItem or shopItem.sell <= 0 then return false end
    
    local totalPrice = amount * shopItem.sell
    local itemName = shopItem.name or ItemType(itemId):getName()
    if doPlayerSellItem(player:getId(), itemId, amount, totalPrice, subType, ignoreEquipped) then
        player:sendTextMessage(MESSAGE_INFO_DESCR, "Sold " .. amount .. "x " .. shopItem.name .. " for " .. (amount * shopItem.sell) .. " gold.")
        return true
    end
    player:sendCancelMessage("You do not have this object.")
    return false
end)

npcType:register()
