-- Lea - Converted from XML to Lua NpcType
-- Original XML: data/npc/Lea.xml
-- Original Script: data/npc/scripts/Lea.lua

local npcName = "Lea"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a lea")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(2000)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 138, lookHead = 59, lookBody = 95, lookLegs = 113, lookFeet = 113})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)


local voices = { {text = 'Utevo vis lux!'} }
npcHandler:addModule(VoiceModule:new(voices))

-- MASTER SPELL CONFIGURATION
-- Vocation {1, 5} = Sorcerer
local spells = {
    -- ATTACK SPELLS
    {name = 'Energy Beam',          price = 1000, level = 23, vocations = {1, 5}, type = 'attack'},
    {name = 'Energy Wave',          price = 2500, level = 38, vocations = {1, 5}, type = 'attack'},
    {name = 'Fire Wave',            price = 850,  level = 18, vocations = {1, 5}, type = 'attack'},
    {name = 'Great Energy Beam',    price = 1800, level = 29, vocations = {1, 5}, type = 'attack'},

    -- HEALING SPELLS
    {name = 'Cure Poison',          price = 150,  level = 10, vocations = {1, 5}, type = 'healing'},
    {name = 'Intense Healing',      price = 350,  level = 20, vocations = {1, 5}, type = 'healing'},
    {name = 'Light Healing',        price = 0,    level = 8,  vocations = {1, 5}, type = 'healing'},
    {name = 'Ultimate Healing',     price = 1000, level = 30, vocations = {1, 5}, type = 'healing'},

    -- SUPPORT SPELLS (Instant Buffs/Summons/Utility)
    {name = 'Creature Illusion',    price = 1000, level = 23, vocations = {1, 5}, type = 'support'},
    {name = 'Find Person',          price = 80,   level = 8,  vocations = {1, 5}, type = 'support'},
    {name = 'Great Light',          price = 500,  level = 13, vocations = {1, 5}, type = 'support'},
    {name = 'Invisibility',            price = 2000, level = 35, vocations = {1, 5}, type = 'support'},
    {name = 'Light',                price = 0,    level = 8,  vocations = {1, 5}, type = 'support'},
    {name = 'Magic Shield',         price = 450,  level = 14, vocations = {1, 5}, type = 'support'},
    {name = 'Summon Creature',      price = 2000, level = 25, vocations = {1, 5}, type = 'support'},

    -- CONJURE SPELLS (Runes - Renamed to match spells.xml)
    {name = 'Destroy Field Rune',      price = 700,  level = 17, vocations = {1, 5}, type = 'conjure'},
    {name = 'Energy Field Rune',       price = 700,  level = 18, vocations = {1, 5}, type = 'conjure'},
    {name = 'Energy Wall Rune',        price = 2500, level = 41, vocations = {1, 5}, type = 'conjure'},
    {name = 'Explosion Rune',          price = 1800, level = 31, vocations = {1, 5}, type = 'conjure'},
    {name = 'Fire Bomb Rune',          price = 1500, level = 27, vocations = {1, 5}, type = 'conjure'},
    {name = 'Fire Field Rune',         price = 500,  level = 15, vocations = {1, 5}, type = 'conjure'},
    {name = 'Fire Wall Rune',          price = 2000, level = 33, vocations = {1, 5}, type = 'conjure'},
    {name = 'Great Fireball Rune',     price = 1200, level = 30, vocations = {1, 5}, type = 'conjure'},
    {name = 'Heavy Magic Missile Rune',price = 1500, level = 25, vocations = {1, 5}, type = 'conjure'},
    {name = 'Light Magic Missile Rune',price = 500,  level = 15, vocations = {1, 5}, type = 'conjure'},
    {name = 'Poison Field Rune',       price = 300,  level = 14, vocations = {1, 5}, type = 'conjure'},
    {name = 'Poison Wall Rune',        price = 1600, level = 29, vocations = {1, 5}, type = 'conjure'},
    {name = 'Stalagmite Rune',         price = 1400, level = 24, vocations = {1, 5}, type = 'conjure'},
    {name = 'Sudden Death Rune',       price = 3000, level = 45, vocations = {1, 5}, type = 'conjure'}
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
    if m:find('attack') or m:find('healing') or m:find('support') or m:find('conjure') then
        local category = 'attack'
        if m:find('healing') then category = 'healing' end
        if m:find('support') then category = 'support' end
        if m:find('conjure') then category = 'conjure' end

        local available = {}
        for _, spell in ipairs(spells) do
            -- Filter logic:
            -- 1. Must match the category (attack/healing/support/conjure)
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

npcHandler:setMessage(MESSAGE_GREET, "Greetings, |PLAYERNAME|.")
npcHandler:setMessage(MESSAGE_FAREWELL, "Take care on your journeys.")

npcHandler:setCallback(CALLBACK_MESSAGE_DEFAULT, creatureSayCallback)
npcHandler:addModule(FocusModule:new(), npcHandler.keywordHandler)

-- FLAVOR KEYWORDS
keywordHandler:addKeyword({'job'}, StdModule.say, {npcHandler = npcHandler, text = "I am the archsorcerer of Carlin. I keep the secrets of our order."})
keywordHandler:addKeyword({'name'}, StdModule.say, {npcHandler = npcHandler, text = "My name is Lea."})
keywordHandler:addKeyword({'time'}, StdModule.say, {npcHandler = npcHandler, text = "Time is a force we sorcerers will master one day."})
keywordHandler:addKeyword({'sorcerer'}, StdModule.say, {npcHandler = npcHandler, text = "Any sorcerer dedicates his whole life to the study of the arcane arts."})
keywordHandler:addKeyword({'power'}, StdModule.say, {npcHandler = npcHandler, text = "We sorcerers wield arcane powers beyond comprehension of men."})
keywordHandler:addKeyword({'arcane'}, StdModule.say, {npcHandler = npcHandler, text = "We sorcerers wield arcane powers beyond comprehension of men."})

-- AUTOMATED SPELL KEYWORD REGISTRATION
-- Sort spells by name length (longest first) to ensure longer spell names match before shorter ones
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
