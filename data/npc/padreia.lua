-- Padreia - Converted from XML to Lua NpcType
-- Original XML: data/npc/Padreia.xml
-- Original Script: data/npc/scripts/Padreia.lua

local npcName = "Padreia"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a padreia")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(2000)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 138, lookBody = 87, lookLegs = 85, lookFeet = 95})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)



-- MASTER SPELL CONFIGURATION
-- Vocations: {2} = Druid, {6} = Elder Druid
local spells = {
    -- ATTACK SPELLS
    {name = 'Ice Wave',             price = 850,   level = 18,  vocations = {2, 6}, type = 'attack'},
    {name = 'Terra Wave',           price = 2500,  level = 38,  vocations = {2, 6}, type = 'attack'},

    -- HEALING SPELLS
    {name = 'Cure Poison',          price = 150,   level = 10,  vocations = {2, 6}, type = 'healing'},
    {name = 'Intense Healing',      price = 350,   level = 20,  vocations = {2, 6}, type = 'healing'},
    {name = 'Light Healing',        price = 0,     level = 8,   vocations = {2, 6}, type = 'healing'},
    {name = 'Ultimate Healing',     price = 1000,  level = 30,  vocations = {2, 6}, type = 'healing'},

    -- SUPPORT SPELLS (Utility/Instant)
    {name = 'Creature Illusion',    price = 1000,  level = 23,  vocations = {2, 6}, type = 'support'},
    {name = 'Find Person',          price = 80,    level = 8,   vocations = {2, 6}, type = 'support'},
    {name = 'Food',                 price = 300,   level = 14,  vocations = {2, 6}, type = 'support'},
    {name = 'Great Light',          price = 500,   level = 13,  vocations = {2, 6}, type = 'support'},
    {name = 'Invisibility',            price = 2000,  level = 35,  vocations = {2, 6}, type = 'support'},
    {name = 'Light',                price = 0,     level = 8,   vocations = {2, 6}, type = 'support'},
    {name = 'Magic Shield',         price = 450,   level = 14,  vocations = {2, 6}, type = 'support'},
    {name = 'Summon Creature',      price = 2000,  level = 25,  vocations = {2, 6}, type = 'support'},

    -- CONJURE SPELLS (Runes)
    {name = 'Avalanche Rune',          price = 1200,  level = 30,  vocations = {2, 6}, type = 'conjure'},
    {name = 'Chameleon Rune',          price = 1300,  level = 27,  vocations = {2, 6}, type = 'conjure'},
    {name = 'Convince Creature Rune',  price = 800,   level = 16,  vocations = {2, 6}, type = 'conjure'},
    {name = 'Cure Poison Rune',        price = 600,   level = 15,  vocations = {2, 6}, type = 'conjure'},
    {name = 'Destroy Field Rune',      price = 700,   level = 17,  vocations = {2, 6}, type = 'conjure'},
    {name = 'Energy Field Rune',       price = 700,   level = 18,  vocations = {2, 6}, type = 'conjure'},
    {name = 'Energy Wall Rune',        price = 2500,  level = 41,  vocations = {2, 6}, type = 'conjure'},
    {name = 'Explosion Rune',          price = 1800,  level = 31,  vocations = {2, 6}, type = 'conjure'},
    {name = 'Fire Bomb Rune',          price = 1500,  level = 27,  vocations = {2, 6}, type = 'conjure'},
    {name = 'Fire Field Rune',         price = 500,   level = 15,  vocations = {2, 6}, type = 'conjure'},
    {name = 'Fire Wall Rune',          price = 2000,  level = 33,  vocations = {2, 6}, type = 'conjure'},
    {name = 'Heavy Magic Missile Rune',price = 1500,  level = 25,  vocations = {2, 6}, type = 'conjure'},
    {name = 'Intense Healing Rune',    price = 600,   level = 15,  vocations = {2, 6}, type = 'conjure'},
    {name = 'Light Magic Missile Rune',price = 500,   level = 15,  vocations = {2, 6}, type = 'conjure'},
    {name = 'Poison Field Rune',       price = 300,   level = 14,  vocations = {2, 6}, type = 'conjure'},
    {name = 'Poison Wall Rune',        price = 1600,  level = 29,  vocations = {2, 6}, type = 'conjure'},
    {name = 'Stalagmite Rune',         price = 1400,  level = 24,  vocations = {2, 6}, type = 'conjure'},
    {name = 'Ultimate Healing Rune',   price = 1500,  level = 24,  vocations = {2, 6}, type = 'conjure'}
}

-- Helper function to check vocation
local function hasVocation(player, allowedList)
    local pid = player:getVocation():getId()
    for _, vid in ipairs(allowedList) do
        if pid == vid then return true end
    end
    return false
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

    -- COUGH SYRUP & THE EXTERMINATOR QUEST
    if m:find("cough syrup") then
        npcHandler:say("Do you want to buy a bottle of cough syrup for 50 gold?", cid)
        npcHandler.topic[cid] = 10
        return true
    end

    if m:find("mission") then
        if player:getStorageValue(Storage.TibiaTales.TheExterminator) == -1 then
            npcHandler:say({
                'Oh |PLAYERNAME|, thank god you came to me. Last night, I had a vision about an upcoming plague here in Carlin. ...',
                'It will originate from slimes that will swarm out of the sewers and infect every citizen with a deadly disease. Are you willing to help me save Carlin?'
            }, cid)
            npcHandler.topic[cid] = 11
        elseif player:getStorageValue(Storage.TibiaTales.TheExterminator) == 1 then
            npcHandler:say('You MUST find that slime pool immediately or life here in Carlin will not be the same anymore.', cid)
        elseif player:getStorageValue(Storage.TibiaTales.TheExterminator) == 2 then
            local itemId = {2150, 2149, 2147, 2146}
            for i = 1, #itemId do player:addItem(itemId[i], 1) end
            player:setStorageValue(Storage.TibiaTales.TheExterminator, 3)
            npcHandler:say('You did it! Even if only few of the Carliners will ever know about that, you saved all of their lives. Here, take this as a reward. Farewell!', cid)
        else
            npcHandler:say('Maybe the guards have something to do for you or know someone who could need some help.', cid)
        end
        return true
    end

    -- CONFIRMATIONS
    if m == "yes" then
        if npcHandler.topic[cid] == 10 then -- Cough Syrup
            if player:removeMoneyNpc(50) then
                npcHandler:say("Thank you. Here it is.", cid)
                player:addItem(4839, 1)
            else
                npcHandler:say("You don't have enough money.", cid)
            end
            npcHandler.topic[cid] = 0
            return true
        elseif npcHandler.topic[cid] == 11 then -- Start Quest
            player:addItem(8205, 1)
            player:setStorageValue(Storage.TibiaTales.TheExterminator, 1)
            npcHandler:say({
                'I knew I could count on you. Take this highly intensified vermin poison. In my vision, I saw some kind of \'pool\' where these slimes came from. ...',
                'Pour the poison in the water to stop the demise of Carlin. Tell me about your mission after you fulfilled your task.'
            }, cid)
            npcHandler.topic[cid] = 0
            return true
        end
    end

    if m == "no" then
        if npcHandler.topic[cid] == 10 then
            npcHandler:say("Then no.", cid)
            npcHandler.topic[cid] = 0
        elseif npcHandler.topic[cid] == 11 then
            npcHandler:say('Then the downfall of Carlin is inescapable. Please think about it. You know where to find me.', cid)
            npcHandler.topic[cid] = 0
        end
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

npcHandler:setMessage(MESSAGE_GREET, "Welcome to our humble guild, wanderer. May I be of any assistance to you?")
npcHandler:setMessage(MESSAGE_FAREWELL, "Farewell.")
npcHandler:setMessage(MESSAGE_WALKAWAY, "Farewell.")

npcHandler:setCallback(CALLBACK_MESSAGE_DEFAULT, creatureSayCallback)
npcHandler:addModule(FocusModule:new(), npcHandler.keywordHandler)

-- FLAVOR KEYWORDS
keywordHandler:addKeyword({'job'}, StdModule.say, {npcHandler = npcHandler, text = "I am the grand druid of Carlin. I am responsible for the guild, the fields, and our citizens' health."})
keywordHandler:addKeyword({'magic'}, StdModule.say, {npcHandler = npcHandler, text = "Every druid is able to learn the numerous spells of our craft."})
keywordHandler:addKeyword({'name'}, StdModule.say, {npcHandler = npcHandler, text = "I am Padreia, grand druid of our fine city."})
keywordHandler:addKeyword({'druids'}, StdModule.say, {npcHandler = npcHandler, text = "We are druids, preservers of life. Our magic is about defence, healing, and nature."})

-- AUTOMATED SPELL KEYWORD REGISTRATION
-- Sort spells by name length (longest first) to ensure longer spell names match before shorter ones
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
