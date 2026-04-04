-- Tristan - Converted from XML to Lua NpcType
-- Original XML: data/npc/Tristan.xml
-- Original Script: data/npc/scripts/Tristan.lua

local npcName = "Tristan"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a tristan")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(2000)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 131, lookHead = 79, lookBody = 47, lookLegs = 48, lookFeet = 38, lookAddons = 1})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)



-- MASTER SPELL CONFIGURATION
-- Vocations: {4} = Knight, {8} = Elite Knight
local spells = {
    -- ATTACK SPELLS
    {name = 'Berserk',              price = 2500,  level = 35,  vocations = {4, 8}, type = 'attack'},
    {name = 'Fierce Berserk',       price = 7500,  level = 90,  vocations = {4, 8}, type = 'attack'},
    {name = 'Front Sweep',          price = 4000,  level = 70,  vocations = {4, 8}, type = 'attack'},
    {name = 'Groundshaker',         price = 1500,  level = 33,  vocations = {4, 8}, type = 'attack'},
    {name = 'Whirlwind Throw',      price = 1500,  level = 28,  vocations = {4, 8}, type = 'attack'},
    {name = 'Brutal Strike',        price = 1000,   level = 16,  vocations = {4, 8}, type = 'attack'},
    {name = 'Annihilation',         price = 20000,   level = 110,  vocations = {4, 8}, type = 'attack'},
    {name = 'Inflict Wound',         price = 2500,   level = 40,  vocations = {4, 8}, type = 'attack'},

    -- HEALING SPELLS
    {name = 'Cure Poison',          price = 150,   level = 10,  vocations = {4, 8}, type = 'healing'},
    {name = 'Wound Cleansing',      price = 0,     level = 8,   vocations = {4, 8}, type = 'healing'},
    {name = 'Intense Wound Cleansing', price = 6000, level = 80, vocations = {4, 8}, type = 'healing'},
    {name = 'Recovery',             price = 4000,  level = 50,  vocations = {4, 8}, type = 'healing'},
    {name = 'Intense Recovery',     price = 10000, level = 100, vocations = {4, 8}, type = 'healing'},

    -- SUPPORT SPELLS
    {name = 'Charge',               price = 1300,  level = 25,  vocations = {4, 8}, type = 'support'},
    {name = 'Find Person',          price = 80,    level = 8,   vocations = {4, 8}, type = 'support'},
    {name = 'Great Light',          price = 500,   level = 13,  vocations = {4, 8}, type = 'support'},
    {name = 'Haste',                price = 600,   level = 14,  vocations = {4, 8}, type = 'support'},
    {name = 'Levitate',             price = 500,   level = 12,  vocations = {4, 8}, type = 'support'},
    {name = 'Light',                price = 0,     level = 8,   vocations = {4, 8}, type = 'support'},
    {name = 'Magic Rope',           price = 200,   level = 9,   vocations = {4, 8}, type = 'support'}
    -- Summon Skullfrost removed
}

-- Helper function to check vocation
local function hasVocation(player, allowedList)
    local pid = player:getVocation():getId()
    for _, vid in ipairs(allowedList) do
        if pid == vid then return true end
    end
    return false
end

-- Helper to check if player is a Knight (for conversation flow)
local function isKnight(player)
    local pid = player:getVocation():getId()
    return pid == 4 or pid == 8
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

    -- GREETING INFO (Checking Vocation)
    if m == 'spells' then
        if not isKnight(player) then
            npcHandler:say("Sorry, I only teach knights.", cid)
            return true
        end
        npcHandler:say('I can teach you {attack spells}, {healing spells} and {support spells}.', cid)
        return true
    end

    -- DYNAMIC CATEGORY FILTERING
    if m == 'attack spells' or m == 'attack' or m == 'healing spells' or m == 'healing' or m == 'support spells' or m == 'support' then
        
        -- Check Vocation again for categories
        if not isKnight(player) then
            npcHandler:say("Sorry, I only teach knights.", cid)
            return true
        end

        local category = 'attack'
        if m:find('healing') then category = 'healing' end
        if m:find('support') then category = 'support' end

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

npcHandler:setMessage(MESSAGE_GREET, "Greetings, |PLAYERNAME|. I can teach you the art of combat.")
npcHandler:setMessage(MESSAGE_FAREWELL, "Farewell.")

npcHandler:setCallback(CALLBACK_MESSAGE_DEFAULT, creatureSayCallback)
npcHandler:addModule(FocusModule:new(), npcHandler.keywordHandler)

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
