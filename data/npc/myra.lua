-- Myra - Converted from XML to Lua NpcType
-- Original XML: data/npc/Myra.xml
-- Original Script: data/npc/scripts/Myra.lua

local npcName = "Myra"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a myra")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(2000)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 138, lookHead = 58, lookBody = 19, lookFeet = 132, lookAddons = 3})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)


local QUEST = Storage.OutfitQuest.MageSummoner.AddonHatCloak

-- MASTER SPELL CONFIGURATION
-- Vocation {1, 5} = Sorcerer
local spells = {
    -- ATTACK SPELLS
    {name = 'Apprentice\'s Strike',    price = 0,     level = 8,   vocations = {1, 5}, type = 'attack'},
    {name = 'Death Strike',            price = 800,   level = 16,  vocations = {1, 5}, type = 'attack'},
    {name = 'Energy Beam',             price = 1000,  level = 23,  vocations = {1, 5}, type = 'attack'},
    {name = 'Energy Strike',           price = 800,   level = 12,  vocations = {1, 5}, type = 'attack'},
    {name = 'Energy Wave',             price = 2500,  level = 38,  vocations = {1, 5}, type = 'attack'},
    {name = 'Fire Wave',               price = 850,   level = 18,  vocations = {1, 5}, type = 'attack'},
    {name = 'Flame Strike',            price = 800,   level = 14,  vocations = {1, 5}, type = 'attack'},
    {name = 'Great Energy Beam',       price = 1800,  level = 29,  vocations = {1, 5}, type = 'attack'},
    {name = 'Ice Strike',              price = 800,   level = 15,  vocations = {1, 5}, type = 'attack'},
    {name = 'Terra Strike',            price = 800,   level = 13,  vocations = {1, 5}, type = 'attack'},
    {name = 'Strong Energy Strike',         price = 7500,   level = 80,  vocations = {1, 5}, type = 'attack'},
    {name = 'Strong Flame Strike',         price = 6000,   level = 70,  vocations = {1, 5}, type = 'attack'},
    {name = 'Curse',         price = 6000,   level = 75,  vocations = {1, 5}, type = 'attack'},
    {name = 'Electrify',         price = 2500,   level = 34,  vocations = {1, 5}, type = 'attack'},
    {name = 'Ignite',         price = 1500,   level = 26,  vocations = {1, 5}, type = 'attack'},

    -- HEALING SPELLS
    {name = 'Cure Poison',             price = 150,   level = 10,  vocations = {1, 5}, type = 'healing'},
    {name = 'Intense Healing',         price = 350,   level = 20,  vocations = {1, 5}, type = 'healing'},
    {name = 'Light Healing',           price = 0,     level = 8,   vocations = {1, 5}, type = 'healing'},
    {name = 'Ultimate Healing',        price = 1000,  level = 30,  vocations = {1, 5}, type = 'healing'},

    -- SUPPORT SPELLS
    {name = 'Creature Illusion',       price = 1000,  level = 23,  vocations = {1, 5}, type = 'support'},
    {name = 'Find Person',             price = 80,    level = 8,   vocations = {1, 5}, type = 'support'},
    {name = 'Great Light',             price = 500,   level = 13,  vocations = {1, 5}, type = 'support'},
    {name = 'Haste',                   price = 600,   level = 14,  vocations = {1, 5}, type = 'support'},
    {name = 'Invisibility',               price = 2000,  level = 35,  vocations = {1, 5}, type = 'support'},
    {name = 'Levitate',                price = 500,   level = 12,  vocations = {1, 5}, type = 'support'},
    {name = 'Light',                   price = 0,     level = 8,   vocations = {1, 5}, type = 'support'},
    {name = 'Magic Rope',              price = 200,   level = 9,   vocations = {1, 5}, type = 'support'},
    {name = 'Magic Shield',            price = 450,   level = 14,  vocations = {1, 5}, type = 'support'},
    {name = 'Strong Haste',            price = 1300,  level = 20,  vocations = {1, 5}, type = 'support'},
    {name = 'Summon Creature',         price = 2000,  level = 25,  vocations = {1, 5}, type = 'support'},
    {name = 'Ultimate Light',          price = 1600,  level = 26,  vocations = {1, 5}, type = 'support'},

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
    if not npcHandler:isFocused(cid) then
        return false
    end

    local player = Player(cid)
    local storage = player:getStorageValue(QUEST)
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
    if m:find('attack') or m:find('healing') or m:find('support') or m:find('conjure') then
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

    -- QUEST LOGIC
    if m == 'outfit' then
        npcHandler:say('This Tiara is an award by the academy of Edron in recognition of my service here.', cid)
        return true
    end

    if m == 'tiara' and storage < 1 then
        npcHandler:say(
            'Well... maybe, if you help me a little, I could convince the academy of Edron that you are a valuable help here and deserve an award too. How about it?',
            cid
        )
        npcHandler.topic[cid] = 1
        return true
    end

    -- ACCEPT QUEST
    if npcHandler.topic[cid] == 1 then
        if m == 'yes' then
            npcHandler:say({
                'Okay, great! You see, I need a few magical ingredients which I\'ve run out of.',
                'First of all, please bring me 70 bat wings.',
                'Then, I urgently need 20 pieces of red cloth.',
                'Oh, and also, please bring me 40 pieces of ape fur.',
                'After that, I need 35 holy orchids.',
                'Then, 10 spools of spider silk yarn, 60 lizard scales and 40 red dragon scales.',
                'I also need 15 ounces of magic sulphur and 30 ounces of vampire dust.',
                'Did you understand everything I told you and are willing to handle this task?'
            }, cid)
            npcHandler.topic[cid] = 2
        else
            npcHandler:say('Maybe another time then.', cid)
            npcHandler.topic[cid] = 0
        end
        return true
    end

    if npcHandler.topic[cid] == 2 then
        if m == 'yes' then
            player:setStorageValue(QUEST, 1)
            npcHandler:say('Fine! Let\'s start with the 70 bat wings. I really feel uncomfortable out there in the jungle.', cid)
        end
        npcHandler.topic[cid] = 0
        return true
    end

    -- STEP 1 – BAT WINGS
    if m == 'bat wings' and storage == 1 then
        npcHandler:say('Oh, did you bring the 70 bat wings for me?', cid)
        npcHandler.topic[cid] = 10
        return true
    end

    if npcHandler.topic[cid] == 10 and m == 'yes' then
        if not player:removeItem(5894, 70) then
            npcHandler:say('You don\'t have them.', cid)
            npcHandler.topic[cid] = 0
            return true
        end
        player:setStorageValue(QUEST, 2)
        npcHandler:say('Thank you! Now, please bring me 20 pieces of red cloth.', cid)
        npcHandler.topic[cid] = 0
        return true
    end

    -- STEP 2 – RED CLOTH
    if m == 'red cloth' and storage == 2 then
        npcHandler:say('Have you found 20 pieces of red cloth?', cid)
        npcHandler.topic[cid] = 20
        return true
    end

    if npcHandler.topic[cid] == 20 and m == 'yes' then
        if not player:removeItem(5911, 20) then
            npcHandler:say('You don\'t have them.', cid)
            npcHandler.topic[cid] = 0
            return true
        end
        player:setStorageValue(QUEST, 3)
        npcHandler:say('Great! Now bring me 40 pieces of ape fur.', cid)
        npcHandler.topic[cid] = 0
        return true
    end

    -- STEP 3 – APE FUR
    if m == 'ape fur' and storage == 3 then
        npcHandler:say('Were you able to retrieve 40 pieces of ape fur?', cid)
        npcHandler.topic[cid] = 30
        return true
    end

    if npcHandler.topic[cid] == 30 and m == 'yes' then
        if not player:removeItem(5883, 40) then
            npcHandler:say('You don\'t have them.', cid)
            npcHandler.topic[cid] = 0
            return true
        end
        player:setStorageValue(QUEST, 4)
        npcHandler:say('Nice job. Now bring me 35 holy orchids.', cid)
        npcHandler.topic[cid] = 0
        return true
    end

    -- STEP 4 – HOLY ORCHIDS
    if m == 'holy orchids' and storage == 4 then
        npcHandler:say('Did you convince the elves to give you 35 holy orchids?', cid)
        npcHandler.topic[cid] = 40
        return true
    end

    if npcHandler.topic[cid] == 40 and m == 'yes' then
        if not player:removeItem(5922, 35) then
            npcHandler:say('You don\'t have them.', cid)
            npcHandler.topic[cid] = 0
            return true
        end
        player:setStorageValue(QUEST, 5)
        npcHandler:say('Thank god! Now bring me 10 spools of spider silk yarn.', cid)
        npcHandler.topic[cid] = 0
        return true
    end

    -- STEP 5 – SPIDER SILK YARN
    if m == 'spider silk yarn' and storage == 5 then
        npcHandler:say('Oh, did you bring 10 spools of spider silk yarn for me?', cid)
        npcHandler.topic[cid] = 50
        return true
    end

    if npcHandler.topic[cid] == 50 and m == 'yes' then
        if not player:removeItem(5886, 10) then
            npcHandler:say('You don\'t have them.', cid)
            npcHandler.topic[cid] = 0
            return true
        end
        player:setStorageValue(QUEST, 6)
        npcHandler:say('Great! Now bring me 60 lizard scales.', cid)
        npcHandler.topic[cid] = 0
        return true
    end

    -- STEP 6 – LIZARD SCALES
    if m == 'lizard scales' and storage == 6 then
        npcHandler:say('Did you bring 60 lizard scales?', cid)
        npcHandler.topic[cid] = 60
        return true
    end

    if npcHandler.topic[cid] == 60 and m == 'yes' then
        if not player:removeItem(5881, 60) then
            npcHandler:say('You don\'t have them.', cid)
            npcHandler.topic[cid] = 0
            return true
        end
        player:setStorageValue(QUEST, 7)
        npcHandler:say('Excellent! Now bring me 40 red dragon scales.', cid)
        npcHandler.topic[cid] = 0
        return true
    end

    -- STEP 7 – RED DRAGON SCALES
    if m == 'red dragon scales' and storage == 7 then
        npcHandler:say('Have you collected 40 red dragon scales?', cid)
        npcHandler.topic[cid] = 70
        return true
    end

    if npcHandler.topic[cid] == 70 and m == 'yes' then
        if not player:removeItem(5882, 40) then
            npcHandler:say('You don\'t have them.', cid)
            npcHandler.topic[cid] = 0
            return true
        end
        player:setStorageValue(QUEST, 8)
        npcHandler:say('Perfect! Now bring me 15 ounces of magic sulphur.', cid)
        npcHandler.topic[cid] = 0
        return true
    end

    -- STEP 8 – MAGIC SULPHUR
    if m == 'magic sulphur' and storage == 8 then
        npcHandler:say('Did you gather 15 ounces of magic sulphur?', cid)
        npcHandler.topic[cid] = 80
        return true
    end

    if npcHandler.topic[cid] == 80 and m == 'yes' then
        if not player:removeItem(5904, 15) then
            npcHandler:say('You don\'t have it.', cid)
            npcHandler.topic[cid] = 0
            return true
        end
        player:setStorageValue(QUEST, 9)
        npcHandler:say('Wonderful! Now bring me 30 ounces of vampire dust.', cid)
        npcHandler.topic[cid] = 0
        return true
    end

    -- FINAL STEP HAND-OFF
    if m == 'vampire dust' and storage == 9 then
        npcHandler:say('Have you gathered 30 ounces of vampire dust?', cid)
        npcHandler.topic[cid] = 90
        return true
    end

    if npcHandler.topic[cid] == 90 and m == 'yes' then
        if not player:removeItem(5905, 30) then
            npcHandler:say('You don\'t have it.', cid)
            npcHandler.topic[cid] = 0
            return true
        end
        player:setStorageValue(QUEST, 10)
        npcHandler:say(
            'Ah, great. Now go to the academy of Edron and tell Zoltan that I sent you.',
            cid
        )
        npcHandler.topic[cid] = 0
        return true
    end

    return true
end

npcHandler:setMessage(MESSAGE_GREET, 'Greetings, |PLAYERNAME|. If you are looking for sorcerer {spells} don\'t hesitate to ask.')
npcHandler:setMessage(MESSAGE_FAREWELL, 'Farewell, |PLAYERNAME|.')

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
