-- Ustan - Converted from XML to Lua NpcType
-- Original XML: data/npc/Ustan.xml
-- Original Script: data/npc/scripts/Ustan.lua

local npcName = "Ustan"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a ustan")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(2000)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 144, lookBody = 38, lookLegs = 76, lookFeet = 95, lookAddons = 1})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)



-- MASTER SPELL CONFIGURATION
-- Vocations: {2} = Druid, {6} = Elder Druid
local spells = {
    -- ATTACK SPELLS
    {name = 'Energy Strike',        price = 800,   level = 12,  vocations = {2, 6}, type = 'attack'},
    {name = 'Flame Strike',         price = 800,   level = 14,  vocations = {2, 6}, type = 'attack'},
    {name = 'Ice Strike',           price = 800,   level = 15,  vocations = {2, 6}, type = 'attack'},
    {name = 'Ice Wave',             price = 850,   level = 18,  vocations = {2, 6}, type = 'attack'},
    {name = 'Terra Strike',         price = 800,   level = 13,  vocations = {2, 6}, type = 'attack'},
    {name = 'Terra Wave',           price = 2500,  level = 38,  vocations = {2, 6}, type = 'attack'},
    {name = 'Strong Tera Strike',         price = 6000,   level = 70,  vocations = {2, 6}, type = 'attack'},
    {name = 'Strong Ice Strike',           price = 6000,   level = 80,  vocations = {2, 6}, type = 'attack'},
    {name = 'Physical Strike',         price = 800,   level = 16,  vocations = {2, 6}, type = 'attack'},
    {name = 'Strong Ice Wave',           price = 7500,   level = 40,  vocations = {2, 6}, type = 'attack'},

    -- HEALING SPELLS
    {name = 'Cure Poison',          price = 150,   level = 10,  vocations = {2, 6}, type = 'healing'},
    {name = 'Cure Bleeding',          price = 2500,   level = 45,  vocations = {2, 6}, type = 'healing'},
    {name = 'Cure Burning',          price = 2000,   level = 30,  vocations = {2, 6}, type = 'healing'},
    {name = 'Cure Electrification',          price = 1000,   level = 22,  vocations = {2, 6}, type = 'healing'},
    {name = 'Heal Friend',          price = 800,   level = 18,  vocations = {2, 6}, type = 'healing'},
    {name = 'Intense Healing',      price = 350,   level = 20,  vocations = {2, 6}, type = 'healing'},
    {name = 'Light Healing',        price = 0,     level = 8,   vocations = {2, 6}, type = 'healing'},
    {name = 'Mass Healing',         price = 2200,  level = 36,  vocations = {2, 6}, type = 'healing'},
    {name = 'Ultimate Healing',     price = 1000,  level = 30,  vocations = {2, 6}, type = 'healing'},

    -- SUPPORT SPELLS (Instant Utility)
    {name = 'Creature Illusion',    price = 1000,  level = 23,  vocations = {2, 6}, type = 'support'},
    {name = 'Find Person',          price = 80,    level = 8,   vocations = {2, 6}, type = 'support'},
    {name = 'Food',                 price = 300,   level = 14,  vocations = {2, 6}, type = 'support'},
    {name = 'Great Light',          price = 500,   level = 13,  vocations = {2, 6}, type = 'support'},
    {name = 'Haste',                price = 600,   level = 14,  vocations = {2, 6}, type = 'support'},
    {name = 'Invisibility',            price = 2000,  level = 35,  vocations = {2, 6}, type = 'support'},
    {name = 'Levitate',             price = 500,   level = 12,  vocations = {2, 6}, type = 'support'},
    {name = 'Light',                price = 0,     level = 8,   vocations = {2, 6}, type = 'support'},
    {name = 'Magic Rope',           price = 200,   level = 9,   vocations = {2, 6}, type = 'support'},
    {name = 'Magic Shield',         price = 450,   level = 14,  vocations = {2, 6}, type = 'support'},
    {name = 'Strong Haste',         price = 1300,  level = 20,  vocations = {2, 6}, type = 'support'},
    {name = 'Summon Creature',      price = 2000,  level = 25,  vocations = {2, 6}, type = 'support'},
    {name = 'Summon Grovebeast',    price = 50000, level = 200, vocations = {2, 6}, type = 'support'},
    {name = 'Ultimate Light',       price = 1600,  level = 26,  vocations = {2, 6}, type = 'support'},

    -- CONJURE SPELLS (Runes)
    {name = 'Animate Dead Rune',       price = 1200,  level = 27,  vocations = {2, 6}, type = 'conjure'},
    {name = 'Avalanche Rune',          price = 1200,  level = 30,  vocations = {2, 6}, type = 'conjure'},
    {name = 'Chameleon Rune',          price = 1300,  level = 27,  vocations = {2, 6}, type = 'conjure'},
    {name = 'Convince Creature Rune',  price = 800,   level = 16,  vocations = {2, 6}, type = 'conjure'},
    {name = 'Cure Poison Rune',        price = 600,   level = 15,  vocations = {2, 6}, type = 'conjure'},
    {name = 'Destroy Field Rune',      price = 700,   level = 17,  vocations = {2, 6}, type = 'conjure'},
    {name = 'Disintegrate Rune',       price = 900,   level = 21,  vocations = {2, 6}, type = 'conjure'},
    {name = 'Energy Field Rune',       price = 700,   level = 18,  vocations = {2, 6}, type = 'conjure'},
    {name = 'Energy Wall Rune',        price = 2500,  level = 41,  vocations = {2, 6}, type = 'conjure'},
    {name = 'Explosion Rune',          price = 1800,  level = 31,  vocations = {2, 6}, type = 'conjure'},
    {name = 'Fire Bomb Rune',          price = 1500,  level = 27,  vocations = {2, 6}, type = 'conjure'},
    {name = 'Fire Field Rune',         price = 500,   level = 15,  vocations = {2, 6}, type = 'conjure'},
    {name = 'Fire Wall Rune',          price = 2000,  level = 33,  vocations = {2, 6}, type = 'conjure'},
    {name = 'Heavy Magic Missile Rune',price = 1500,  level = 25,  vocations = {2, 6}, type = 'conjure'},
    {name = 'Icicle Rune',             price = 1700,  level = 28,  vocations = {2, 6}, type = 'conjure'},
    {name = 'Intense Healing Rune',    price = 600,   level = 15,  vocations = {2, 6}, type = 'conjure'},
    {name = 'Light Magic Missile Rune',price = 500,   level = 15,  vocations = {2, 6}, type = 'conjure'},
    {name = 'Poison Bomb Rune',        price = 1000,  level = 25,  vocations = {2, 6}, type = 'conjure'},
    {name = 'Poison Field Rune',       price = 300,   level = 14,  vocations = {2, 6}, type = 'conjure'},
    {name = 'Poison Wall Rune',        price = 1600,  level = 29,  vocations = {2, 6}, type = 'conjure'},
    {name = 'Soulfire Rune',           price = 1800,  level = 27,  vocations = {2, 6}, type = 'conjure'},
    {name = 'Stalagmite Rune',         price = 1400,  level = 24,  vocations = {2, 6}, type = 'conjure'},
    {name = 'Stone Shower Rune',       price = 1100,  level = 28,  vocations = {2, 6}, type = 'conjure'},
    {name = 'Ultimate Healing Rune',   price = 1500,  level = 24,  vocations = {2, 6}, type = 'conjure'}
}

local function hasVocation(player, allowedList)
    local pid = player:getVocation():getId()
    for _, vid in ipairs(allowedList) do
        if pid == vid then return true end
    end
    return false
end

local function isDruid(player)
    local pid = player:getVocation():getId()
    return pid == 2 or pid == 6
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

    -- COUGH SYRUP
    if m:find("cough syrup") then
        npcHandler:say("I had some cough syrup a while ago. It was stolen in an ape raid. I fear if you want more cough syrup you will have to buy it in the druids guild in carlin.", cid)
        return true
    end

    -- OUTFIT QUEST
    if m:find("addon") then
        if player:getStorageValue(Storage.OutfitQuest.DruidBodyAddon) < 1 then
            npcHandler:say("Would you like to wear bear paws like I do? Just bring me 50 bear paws and 50 wolf paws and I'll fit them on.", cid)
            player:setStorageValue(Storage.OutfitQuest.DruidBodyAddon, 1)
        end
        return true
    end

    if m:find("paws") then
        if player:getStorageValue(Storage.OutfitQuest.DruidBodyAddon) == 1 then
            npcHandler:say("Have you brought 50 bear paws and 50 wolf paws?", cid)
            npcHandler.topic[cid] = 10
        end
        return true
    end

    if m == "yes" and npcHandler.topic[cid] == 10 then
        if player:getItemCount(5896) >= 50 and player:getItemCount(5897) >= 50 then
            npcHandler:say("Excellent! Like promised, here are your bear paws.", cid)
            player:removeItem(5896, 50)
            player:removeItem(5897, 50)
            player:setStorageValue(Storage.OutfitQuest.DruidBodyAddon, 2)
            player:addOutfitAddon(148, 1)
            player:addOutfitAddon(144, 1)
            player:getPosition():sendMagicEffect(CONST_ME_MAGIC_BLUE)
        else
            npcHandler:say("You don't have enough paws.", cid)
        end
        npcHandler.topic[cid] = 0
        return true
    end

    -- SPELLS DIALOGUE
    if m == 'spells' then
        if not isDruid(player) then npcHandler:say("Sorry, I only teach druids.", cid) return true end
        npcHandler:say('I can teach you {attack spells}, {healing spells}, {support spells} and {conjure spells}.', cid)
        return true
    end

    if m:find('attack') or m:find('healing') or m:find('support') or m:find('conjure') then
        if not isDruid(player) then npcHandler:say("Sorry, I only teach druids.", cid) return true end
        local category = m:find('healing') and 'healing' or m:find('support') and 'support' or m:find('conjure') and 'conjure' or 'attack'
        local available = {}
        for _, spell in ipairs(spells) do
            if spell.type == category and not player:hasLearnedSpell(spell.name) and hasVocation(player, spell.vocations) then
                table.insert(available, spell.name)
            end
        end
        if #available > 0 then npcHandler:say("In this category I have " .. table.concat(available, ", ") .. ".", cid)
        else npcHandler:say("You know all my " .. category .. " spells.", cid) end
        return true
    end

    return true
end

npcHandler:setMessage(MESSAGE_GREET, "Greetings, |PLAYERNAME|. I can teach you the ways of nature.")
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
    {id = 5896, buy = 0, sell = 100, subType = 0, name = "bear paw"},
    {id = 5897, buy = 0, sell = 70, subType = 0, name = "wolf paw"},
    {id = 2129, buy = 0, sell = 100, subType = 0, name = "wolf tooth chain"},
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
