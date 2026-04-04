-- Tamoril - Converted from XML to Lua NpcType
-- Original XML: data/npc/Tamoril.xml
-- Original Script: data/npc/scripts/Tamoril.lua

local npcName = "Tamoril"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a tamoril")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(2000)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 39})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)


local voices = { {text = 'Utevo vis lux!'} }
npcHandler:addModule(VoiceModule:new(voices))

-- MASTER SPELL CONFIGURATION
-- Vocations: 1=Sorc, 5=MS
local spells = {
    -- ATTACK SPELLS
    {name = 'Death Strike',         price = 800,   level = 16,  vocations = {1, 5}, type = 'attack'},
    {name = 'Energy Beam',          price = 1000,  level = 23,  vocations = {1, 5}, type = 'attack'},
    {name = 'Energy Strike',        price = 800,   level = 12,  vocations = {1, 5}, type = 'attack'},
    {name = 'Energy Wave',          price = 2500,  level = 38,  vocations = {1, 5}, type = 'attack'},
    {name = 'Fire Wave',            price = 850,   level = 18,  vocations = {1, 5}, type = 'attack'},
    {name = 'Flame Strike',         price = 800,   level = 14,  vocations = {1, 5}, type = 'attack'},
    {name = 'Great Energy Beam',    price = 1800,  level = 29,  vocations = {1, 5}, type = 'attack'},
    {name = 'Ice Strike',           price = 800,   level = 15,  vocations = {1, 5}, type = 'attack'},
    {name = 'Terra Strike',         price = 800,   level = 13,  vocations = {1, 5}, type = 'attack'},
    {name = 'Strong Energy Strike',         price = 7500,   level = 80,  vocations = {1, 5}, type = 'attack'},
    {name = 'Strong Flame Strike',         price = 6000,   level = 70,  vocations = {1, 5}, type = 'attack'},
    {name = 'Curse',         price = 6000,   level = 75,  vocations = {1, 5}, type = 'attack'},
    {name = 'Electrify',         price = 2500,   level = 34,  vocations = {1, 5}, type = 'attack'},
    {name = 'Ignite',         price = 1500,   level = 26,  vocations = {1, 5}, type = 'attack'},

    -- HEALING SPELLS
    {name = 'Cure Poison',          price = 150,   level = 10,  vocations = {1, 5}, type = 'healing'},
    {name = 'Intense Healing',      price = 350,   level = 20,  vocations = {1, 5}, type = 'healing'},
    {name = 'Light Healing',        price = 0,     level = 8,   vocations = {1, 5}, type = 'healing'},
    {name = 'Ultimate Healing',     price = 1000,  level = 30,  vocations = {1, 5}, type = 'healing'},

    -- SUPPORT SPELLS (Instant/Buffs)
    {name = 'Creature Illusion',    price = 1000,  level = 23,  vocations = {1, 5}, type = 'support'},
    {name = 'Find Person',          price = 80,    level = 8,   vocations = {1, 5}, type = 'support'},
    {name = 'Great Light',          price = 500,   level = 13,  vocations = {1, 5}, type = 'support'},
    {name = 'Haste',                price = 600,   level = 14,  vocations = {1, 5}, type = 'support'},
    {name = 'Invisibility',            price = 2000,  level = 35,  vocations = {1, 5}, type = 'support'},
    {name = 'Levitate',             price = 500,   level = 12,  vocations = {1, 5}, type = 'support'},
    {name = 'Light',                price = 0,     level = 8,   vocations = {1, 5}, type = 'support'},
    {name = 'Magic Rope',           price = 200,   level = 9,   vocations = {1, 5}, type = 'support'},
    {name = 'Magic Shield',         price = 450,   level = 14,  vocations = {1, 5}, type = 'support'},
    {name = 'Strong Haste',         price = 1300,  level = 20,  vocations = {1, 5}, type = 'support'},
    {name = 'Summon Creature',      price = 2000,  level = 25,  vocations = {1, 5}, type = 'support'},
    {name = 'Ultimate Light',       price = 1600,  level = 26,  vocations = {1, 5}, type = 'support'},

    -- CONJURE SPELLS (Runes)
    {name = 'Animate Dead Rune',       price = 1200,  level = 27,  vocations = {1, 5}, type = 'conjure'},
    {name = 'Destroy Field Rune',      price = 700,   level = 17,  vocations = {1, 5}, type = 'conjure'},
    {name = 'Disintegrate Rune',       price = 900,   level = 21,  vocations = {1, 5}, type = 'conjure'},
    {name = 'Energy Bomb Rune',        price = 2300,  level = 37,  vocations = {1, 5}, type = 'conjure'},
    {name = 'Energy Field Rune',       price = 700,   level = 18,  vocations = {1, 5}, type = 'conjure'},
    {name = 'Energy Wall Rune',        price = 2500,  level = 41,  vocations = {1, 5}, type = 'conjure'},
    {name = 'Explosion Rune',          price = 1800,  level = 31,  vocations = {1, 5}, type = 'conjure'},
    {name = 'Fire Bomb Rune',          price = 1500,  level = 27,  vocations = {1, 5}, type = 'conjure'},
    {name = 'Fire Field Rune',         price = 500,   level = 15,  vocations = {1, 5}, type = 'conjure'},
    {name = 'Fire Wall Rune',          price = 2000,  level = 33,  vocations = {1, 5}, type = 'conjure'},
    {name = 'Fireball Rune',           price = 1600,  level = 27,  vocations = {1, 5}, type = 'conjure'},
    {name = 'Great Fireball Rune',     price = 1200,  level = 30,  vocations = {1, 5}, type = 'conjure'},
    {name = 'Heavy Magic Missile Rune',price = 1500,  level = 25,  vocations = {1, 5}, type = 'conjure'},
    {name = 'Light Magic Missile Rune',price = 500,   level = 15,  vocations = {1, 5}, type = 'conjure'},
    {name = 'Magic Wall Rune',         price = 2100,  level = 32,  vocations = {1, 5}, type = 'conjure'},
    {name = 'Poison Field Rune',       price = 300,   level = 14,  vocations = {1, 5}, type = 'conjure'},
    {name = 'Poison Wall Rune',        price = 1600,  level = 29,  vocations = {1, 5}, type = 'conjure'},
    {name = 'Soulfire Rune',           price = 1800,  level = 27,  vocations = {1, 5}, type = 'conjure'},
    {name = 'Stalagmite Rune',         price = 1400,  level = 24,  vocations = {1, 5}, type = 'conjure'},
    {name = 'Sudden Death Rune',       price = 3000,  level = 45,  vocations = {1, 5}, type = 'conjure'},
    {name = 'Thunderstorm Rune',       price = 1100,  level = 28,  vocations = {1, 5}, type = 'conjure'}
}

-- Helper function to check vocation
local function hasVocation(player, allowedList)
    local pid = player:getVocation():getId()
    for _, vid in ipairs(allowedList) do
        if pid == vid then return true end
    end
    return false
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
    end
    
    if player:getLevel() < level then
        npcHandler:say('You must be at least level ' .. level .. ' to learn this spell.', cid)
        return true
    end
    
    if not hasVocation(player, vocations) then
        npcHandler:say('This spell is not for your vocation.', cid)
        return true
    end
    
    if player:getTotalMoney() < price then
        npcHandler:say('You don\'t have enough money. This spell costs ' .. price .. ' gold.', cid)
        return true
    end
    
    pendingSpell[cid] = {name = spellName, price = price}
    
    if price > 0 then
        npcHandler:say('Would you like to purchase the ' .. spellName .. ' spell for ' .. price .. ' gold?', cid)
    else
        npcHandler:say('Would you like to learn the ' .. spellName .. ' spell? It\'s free!', cid)
    end
    
    npcHandler.topic[cid] = 999 
    return true
end

local function creatureSayCallback(cid, type, msg)
    if not npcHandler:isFocused(cid) then return false end

    local player = Player(cid)
    local m = msg:lower()

    -- SPELL CONFIRMATION
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

    -- GREETING INFO
    if m == 'spells' then
        npcHandler:say('I can teach you {attack spells}, {healing spells}, {support spells} and {conjure spells}.', cid)
        return true
    end

    -- DYNAMIC CATEGORY FILTERING
    if m == 'attack spells' or m == 'attack' or m == 'healing spells' or m == 'healing' or m == 'support spells' or m == 'support' or m == 'conjure spells' or m == 'conjure' then
        local category = 'attack'
        if m:find('healing') then category = 'healing' end
        if m:find('support') then category = 'support' end
        if m:find('conjure') then category = 'conjure' end

        local available = {}
        for _, spell in ipairs(spells) do
            -- Filter logic:
            -- 1. Must match the category
            -- 2. Player must NOT have learned it yet
            -- 3. Player must have the correct vocation
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

npcHandler:setMessage(MESSAGE_GREET, "Another pesky mortal who believes his gold outweighs his nutrition value.")
npcHandler:setMessage(MESSAGE_FAREWELL, "Good riddance.")

npcHandler:setCallback(CALLBACK_MESSAGE_DEFAULT, creatureSayCallback)
npcHandler:addModule(FocusModule:new(), npcHandler.keywordHandler)

-- AUTOMATED SPELL KEYWORD REGISTRATION
-- Sort spells by name length (longest first) to ensure longer spell names match before shorter ones
-- This prevents "fireball rune" from matching before "great fireball rune"
local sortedSpells = {}
for _, spell in ipairs(spells) do
    table.insert(sortedSpells, spell)
end
table.sort(sortedSpells, function(a, b) return #a.name > #b.name end)

for _, spell in ipairs(sortedSpells) do
    keywordHandler:addKeyword({spell.name:lower()}, 
        function(cid)
            return offerSpell(cid, spell.name, spell.price, spell.level, spell.vocations)
        end
    )
end


-- Shop items (from XML parameters)
local shopItems = {
    {id = 2033, buy = 0, sell = 250, subType = 0, name = "golden mug"},
    {id = 2158, buy = 0, sell = 5000, subType = 0, name = "blue gem"},
    {id = 2155, buy = 0, sell = 5000, subType = 0, name = "green gem"},
    {id = 2156, buy = 0, sell = 1000, subType = 0, name = "red gem"},
    {id = 2153, buy = 0, sell = 10000, subType = 0, name = "violet gem"},
    {id = 2154, buy = 0, sell = 1000, subType = 0, name = "yellow gem"},
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
