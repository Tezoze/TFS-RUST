-- Ethan - Converted from XML to Lua NpcType
-- Original XML: data/npc/Ethan.xml
-- Original Script: data/npc/scripts/Ethan.lua

local npcName = "Ethan"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a ethan")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(2000)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 153, lookHead = 19, lookBody = 41, lookLegs = 60, lookFeet = 41, lookAddons = 3})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)



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

    -- SUPPORT SPELLS (Utility/Instant)
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
            npcHandler:say("In this category I have " .. table.concat(available, ", ") .. ".", cid)
        else
            npcHandler:say("You have already learned all the " .. category .. " spells I can teach for your vocation.", cid)
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
