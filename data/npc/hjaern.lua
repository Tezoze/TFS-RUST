-- Hjaern - Converted from XML to Lua NpcType
-- Original XML: data/npc/Hjaern.xml
-- Original Script: data/npc/scripts/Hjaern.lua

local npcName = "Hjaern"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a hjaern")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(2000)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 154, lookBody = 94, lookLegs = 95, lookFeet = 114, lookAddons = 3})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)



-- STORAGE CONSTANTS
local Q_ICEISLANDS = Storage.TheIceIslands.Questline
local Q_MISSION02 = Storage.TheIceIslands.Mission02 -- Nibelor 1
local Q_MISSION07 = Storage.TheIceIslands.Mission07 -- Helheim
local Q_MISSION08 = Storage.TheIceIslands.Mission08 -- Contact
local Q_MISSION10 = Storage.TheIceIslands.Mission10 -- Ghostwhisperer
local Q_MISSION11 = Storage.TheIceIslands.Mission11 -- Mines 3
local Q_MISSION12 = Storage.TheIceIslands.Mission12 -- Mines 4
local Q_NORSEMAN = Storage.OutfitQuest.NorsemanAddon

-- MASTER SPELL CONFIGURATION
-- Vocations: {2, 6} = Druid & Elder Druid
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

    -- Errand Mission (Shattered Isles)
    if m:find("errand") or m:find("gold") then
        if player:getStorageValue(Storage.TheShatteredIsles.TheErrand) == 1 then
            npcHandler:say("Oh, so you brought some gold from Eleonore to me?", cid)
            npcHandler.topic[cid] = 10
        end
        return true
    end

    -- Mission Logic (Ice Islands)
    if m:find("mission") then
        local qState = player:getStorageValue(Q_ICEISLANDS)
        if qState == 3 then
            local mState = player:getStorageValue(Q_MISSION02)
            if mState < 1 then
                npcHandler:say({"We could indeed need some help...", "Destroy the ice at certain places to the east..."}, cid)
                player:setStorageValue(Q_MISSION02, 1)
            elseif mState >= 1 and mState < 4 then
                npcHandler:say("You are still working on breaking the ice passages.", cid)
            else
                npcHandler:say("You have completed the mission. Ask Silfind for more.", cid)
                player:setStorageValue(Q_ICEISLANDS, 5)
            end
        elseif qState == 4 then
            -- Player broke all 3 ice passages (pick.lua sets Questline to 4); report completion
            local mState = player:getStorageValue(Q_MISSION02)
            if mState >= 4 then
                npcHandler:say("You have completed the mission. Ask Silfind for more.", cid)
                player:setStorageValue(Q_ICEISLANDS, 5)
            else
                npcHandler:say("You are still working on breaking the ice passages.", cid)
            end
        elseif qState == 29 then
            npcHandler:say("Permission to travel to Helheim granted. Willing to seek the cause of unrest?", cid)
            npcHandler.topic[cid] = 1
        elseif qState == 31 then
            npcHandler:say("The spirits are destroying the evil magic. One more favour?", cid)
            npcHandler.topic[cid] = 2
        elseif qState == 38 then
            npcHandler:say("Mark the obelisks with this charm.", cid)
            player:setStorageValue(Q_ICEISLANDS, 39)
            player:setStorageValue(Q_MISSION11, 2)
            player:setStorageValue(Q_MISSION12, 1)
            player:addItem(7289, 1)
        elseif qState == 39 and player:getStorageValue(Storage.TheIceIslands.Obelisk01) == 5 then
            if player:removeItem(7289, 1) then
                player:setStorageValue(Q_ICEISLANDS, 40)
                player:setStorageValue(Q_NORSEMAN, 1)
                player:addOutfit(251, 0)
                player:addOutfit(252, 0)
                npcHandler:say("Excellent! Take this outfit as a present.", cid)
            end
        else
            -- Player has not completed Mission 1 (Befriending the Musher) - must talk to Iskan in Svargrond first
            if qState < 3 then
                npcHandler:say("You should speak with Iskan in Svargrond first. He might need help with his dogs - become a friend of the musher before I can give you a mission.", cid)
            else
                npcHandler:say("I have no mission for you at the moment.", cid)
            end
        end
        return true
    end

    -- Yes/No for Quests
    if m == "yes" then
        if npcHandler.topic[cid] == 10 then
            if player:removeMoneyNpc(200) then
                npcHandler:say("Eleonore trusts you. Password: 'peg leg'.", cid)
                player:setStorageValue(Storage.TheShatteredIsles.TheErrand, 2)
            end
        elseif npcHandler.topic[cid] == 1 then
            player:setStorageValue(Q_ICEISLANDS, 30)
            player:setStorageValue(Q_MISSION07, 2)
            npcHandler:say("Seek the reason for unrest in Helheim.", cid)
        elseif npcHandler.topic[cid] == 6 then
            if player:removeItem(8111, 1) then
                player:setStorageValue(Storage.WhatAFoolishQuest.CookieDelivery.Hjaern, 1)
                npcHandler:say("The spirits are not amused!", cid)
                npcHandler:releaseFocus(cid)
            end
        end
        npcHandler.topic[cid] = 0
        return true
    end

    -- Shard Trading
    if m:find("shard") then
        local qState = player:getStorageValue(Q_ICEISLANDS)
        if qState >= 40 then
            npcHandler:say("Do you bring frostheart shards?", cid)
            npcHandler.topic[cid] = 3
        end
        return true
    end

    -- Spell Dialogue
    if m == 'spells' then
        npcHandler:say('I can teach you {attack spells}, {healing spells}, {support spells} and {conjure spells}.', cid)
        return true
    end

    if m:find('attack') or m:find('healing') or m:find('support') or m:find('conjure') then
        local category = m:find('healing') and 'healing' or m:find('support') and 'support' or m:find('conjure') and 'conjure' or 'attack'
        local available = {}
        for _, spell in ipairs(spells) do
            if spell.type == category and not player:hasLearnedSpell(spell.name) and hasVocation(player, spell.vocations) then
                table.insert(available, spell.name)
            end
        end
        if #available > 0 then
            npcHandler:say("In this category I have " .. table.concat(available, ", ") .. ".", cid)
        else
            npcHandler:say("You know all my " .. category .. " spells.", cid)
        end
        return true
    end

    return true
end

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
