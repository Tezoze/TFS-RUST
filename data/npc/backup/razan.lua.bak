-- Razan - Converted from XML to Lua NpcType
-- Original XML: data/npc/Razan.xml
-- Original Script: data/npc/scripts/Razan.lua

local npcName = "Razan"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a razan")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(2000)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 146, lookHead = 19, lookBody = 19, lookLegs = 9, lookFeet = 58, lookAddons = 3})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)



-- MASTER SPELL CONFIGURATION
-- Vocations: {3, 7} = Paladin/RP, {4, 8} = Knight/EK
local spells = {
    -- KNIGHT SPELLS
    {name = 'Berserk',              price = 2500,  level = 35,  vocations = {4, 8}, type = 'attack'},
    {name = 'Charge',               price = 1300,  level = 25,  vocations = {4, 8}, type = 'support'},
    {name = 'Fierce Berserk',       price = 7500,  level = 90,  vocations = {4, 8}, type = 'attack'},
    {name = 'Front Sweep',          price = 4000,  level = 70,  vocations = {4, 8}, type = 'attack'},
    {name = 'Groundshaker',         price = 1500,  level = 33,  vocations = {4, 8}, type = 'attack'},
    {name = 'Whirlwind Throw',      price = 1500,  level = 28,  vocations = {4, 8}, type = 'attack'},
    {name = 'Brutal Strike',        price = 1000,   level = 16,  vocations = {4, 8}, type = 'attack'},
    {name = 'Annihilation',         price = 20000,   level = 110,  vocations = {4, 8}, type = 'attack'},
    {name = 'Inflict Wound',         price = 2500,   level = 40,  vocations = {4, 8}, type = 'attack'},

    -- PALADIN SPELLS
    {name = 'Divine Caldera',       price = 3000,  level = 50,  vocations = {3, 7}, type = 'attack'},
    {name = 'Divine Healing',       price = 3000,  level = 35,  vocations = {3, 7}, type = 'healing'},
    {name = 'Divine Missile',       price = 1800,  level = 40,  vocations = {3, 7}, type = 'attack'},
    {name = 'Ethereal Spear',       price = 1100,  level = 23,  vocations = {3, 7}, type = 'attack'},
    {name = 'Intense Healing',      price = 350,   level = 20,  vocations = {3, 7}, type = 'healing'},
    {name = 'Light Healing',        price = 0,     level = 8,   vocations = {3, 7}, type = 'healing'},
    {name = 'Wound Cleansing',      price = 0,     level = 8,   vocations = {4, 8}, type = 'healing'},
    {name = 'Intense Wound Cleansing', price = 6000, level = 80, vocations = {4, 8}, type = 'healing'},
    {name = 'Recovery',             price = 4000,  level = 50,  vocations = {3, 7, 4, 8}, type = 'healing'},
    {name = 'Intense Recovery',     price = 10000, level = 100, vocations = {3, 7, 4, 8}, type = 'healing'},
    {name = 'Salvation',     price = 8000, level = 60, vocations = {3, 7}, type = 'healing'},
    {name = 'Cure Curse',     price = 6000, level = 80, vocations = {3, 7}, type = 'healing'},
    {name = 'Holy Flash',       price = 7500,  level = 70,  vocations = {3, 7}, type = 'attack'},
    {name = 'Strong Ethereal Spear',       price = 10000,  level = 90,  vocations = {3, 7}, type = 'attack'},

    -- SHARED SPELLS (Paladin & Knight)
    {name = 'Cure Poison',          price = 150,   level = 10,  vocations = {3, 7, 4, 8}, type = 'healing'},
    {name = 'Find Person',          price = 80,    level = 8,   vocations = {3, 7, 4, 8}, type = 'support'},
    {name = 'Great Light',          price = 500,   level = 13,  vocations = {3, 7, 4, 8}, type = 'support'},
    {name = 'Haste',                price = 600,   level = 14,  vocations = {3, 7, 4, 8}, type = 'support'},
    {name = 'Levitate',             price = 500,   level = 12,  vocations = {3, 7, 4, 8}, type = 'support'},
    {name = 'Light',                price = 0,     level = 8,   vocations = {3, 7, 4, 8}, type = 'support'},
    {name = 'Magic Rope',           price = 200,   level = 9,   vocations = {3, 7, 4, 8}, type = 'support'},

    -- SUPPORT SPELLS (Utility/Instant)
    {name = 'Cancel Invisibility',  price = 1600,  level = 26,  vocations = {3, 7}, type = 'support'},

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

-- ORIENTAL ADDON CONFIG
local addonConfig = {
    ['ape fur'] = {itemId = 5883, count = 100, storageValue = 1,
        text = {'Have you really managed to fulfil the task and brought me 100 pieces of ape fur?',
                'Only ape fur is good enough to touch the feet of our Caliph.',
                'Ahhh, this softness! I\'m impressed, |PLAYERNAME|. You\'re on the best way to earn that turban. Now, please retrieve 100 fish fins.'}},
    ['fish fins'] = {itemId = 5895, count = 100, storageValue = 2,
        text = {'Were you able to discover the undersea race and retrieved 100 fish fins?',
                'I really wonder what the explorer society is up to. Actually I have no idea how they managed to dive unterwater.',
                'I never thought you\'d make it, |PLAYERNAME|. Now we only need two enchanted chicken wings to start our waterwalking test!'}},
    ['enchanted chicken wings'] = {itemId = 5891, count = 2, storageValue = 3,
        text = {'Were you able to get hold of two enchanted chicken wings?',
                'Enchanted chicken wings are actually used to make boots of haste, so they could be magically extracted again. Djinns are said to be good at that.',
                'Great, thank you very much. Just bring me 100 pieces of blue cloth now and I will happily show you how to make a turban.'}},
    ['blue cloth'] = {itemId = 5912, count = 100, storageValue = 4,
        text = {'Ah, have you brought the 100 pieces of blue cloth?',
                'It\'s a great material for turbans.',
                'Ah! Congratulations - even if you are not a true weaponmaster, you surely deserve to wear this turban. Here, I\'ll tie it for you.'}}
}

local function hasVocation(player, allowedList)
    local pid = player:getVocation():getId()
    for _, vid in ipairs(allowedList) do
        if pid == vid then return true end
    end
    return false
end

local function isValidVocation(player)
    local pid = player:getVocation():getId()
    return pid == 3 or pid == 7 or pid == 4 or pid == 8
end

local pendingSpell = {}
local pendingAddonItem = {}

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

    -- ORIENTAL OUTFIT QUEST
    if m:find('outfit') then
        if player:getSex() == PLAYERSEX_FEMALE then
            npcHandler:say('My turban? I know something better for a pretty girl like you. Why don\'t you go talk to Miraia?', cid)
        else
            npcHandler:say('My turban? Only oriental weaponmasters may wear it after having completed a difficult task.', cid)
        end
        return true
    end

    if m:find('task') then
        if player:getSex() == PLAYERSEX_FEMALE then
            npcHandler:say('If you are looking for a job, ask Miraia.', cid)
            return true
        end
        if player:getStorageValue(Storage.OutfitQuest.secondOrientalAddon) < 1 then
            npcHandler:say('You mean, you would like to prove that you deserve to wear such a turban?', cid)
            npcHandler.topic[cid] = 1
        end
        return true
    end

    if addonConfig[m] and npcHandler.topic[cid] == 0 then
        if player:getStorageValue(Storage.OutfitQuest.secondOrientalAddon) == addonConfig[m].storageValue then
            npcHandler:say(addonConfig[m].text[1], cid)
            npcHandler.topic[cid] = 3
            pendingAddonItem[cid] = m
        else
            npcHandler:say(addonConfig[m].text[2], cid)
        end
        return true
    end

    if m == 'yes' then
        if npcHandler.topic[cid] == 1 then
            npcHandler:say({'Alright, then listen... First, bring me 100 pieces of {ape fur}. Second, 100 {fish fins}. Third, two {enchanted chicken wings}. Lastly, 100 pieces of {blue cloth}.', 'willing to handle this task?'}, cid)
            npcHandler.topic[cid] = 2
            return true
        elseif npcHandler.topic[cid] == 2 then
            player:setStorageValue(Storage.OutfitQuest.DefaultStart, 1)
            player:setStorageValue(Storage.OutfitQuest.secondOrientalAddon, 1)
            npcHandler:say('Excellent! Come back when you have 100 pieces of ape fur.', cid)
            npcHandler.topic[cid] = 0
            return true
        elseif npcHandler.topic[cid] == 3 then
            local key = pendingAddonItem[cid]
            if key and addonConfig[key] then
                local cfg = addonConfig[key]
                if player:removeItem(cfg.itemId, cfg.count) then
                    player:setStorageValue(Storage.OutfitQuest.secondOrientalAddon, player:getStorageValue(Storage.OutfitQuest.secondOrientalAddon) + 1)
                    if player:getStorageValue(Storage.OutfitQuest.secondOrientalAddon) == 5 then
                        player:addOutfitAddon(146, 2)
                        player:addOutfitAddon(150, 2)
                        player:getPosition():sendMagicEffect(CONST_ME_MAGIC_BLUE)
                    end
                    npcHandler:say(cfg.text[3], cid)
                else
                    npcHandler:say('You do not have the items.', cid)
                end
            end
            npcHandler.topic[cid] = 0
            return true
        end
    end

    -- SPELL TEACHING
    if m == 'spells' then
        if not isValidVocation(player) then npcHandler:say("Sorry, I only teach knights and paladins.", cid) return true end
        npcHandler:say('I can teach you {attack spells}, {healing spells}, {support spells} and {conjure spells}.', cid)
        return true
    end

    if m:find('attack') or m:find('healing') or m:find('support') or m:find('conjure') then
        if not isValidVocation(player) then npcHandler:say("Sorry, I only teach knights and paladins.", cid) return true end
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

npcHandler:setMessage(MESSAGE_GREET, 'Greetings |PLAYERNAME|. What leads you to me?')
npcHandler:setMessage(MESSAGE_FAREWELL, 'Daraman\'s blessings.')
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

npcHandler:addModule(FocusModule:new())
npcType:register()
