-- Rahkem - Converted from XML to Lua NpcType
-- Original XML: data/npc/Rahkem.xml
-- Original Script: data/npc/scripts/Rahkem.lua

local npcName = "Rahkem"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a rahkem")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(2000)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 130, lookBody = 77, lookLegs = 87, lookFeet = 116})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)


local voices = { {text = 'Please, not so loud, not so loud. Some of us are trying to rest in peace here.'} }
npcHandler:addModule(VoiceModule:new(voices))

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

    -- SUPPORT SPELLS (Utility/Instant)
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

    -- TWIST OF FATE BLESSING
    if m:find("twist") or m:find("fate") then
        local pvpBlessCost = StdModule.calculateRegularBlessingCost(player:getLevel())
        npcHandler:say({'This is a special blessing I can bestow upon you once you have obtained at least one of the other blessings. ...', 'It prevents you from losing your other blessings as well as the amulet of loss... Would you like to receive that protection for ' .. pvpBlessCost .. ' gold?'}, cid)
        npcHandler.topic[cid] = 1
        return true
    end

    -- ADVENTURER STONE
    if m:find("adventurer") and m:find("stone") then
        if player:getItemById(18559, true) then npcHandler:say('Keep your adventurer\'s stone well.', cid)
        elseif player:getStorageValue(Storage.AdventurersGuild.FreeStone.Rahkem) ~= 1 then
            npcHandler:say('Ah, you want to replace your adventurer\'s stone for free?', cid)
            npcHandler.topic[cid] = 2
        else
            npcHandler:say('Ah, you want to replace your adventurer\'s stone for 30 gold?', cid)
            npcHandler.topic[cid] = 3
        end
        return true
    end

    -- BLESSED STAKE QUEST
    if m:find("stake") then
        local stakeStorage = player:getStorageValue(Storage.FriendsAndTraders.TheBlessedStake)
        if stakeStorage == 8 then
            if player:getItemCount(5941) > 0 then npcHandler:say('Yes, I was informed what to do. Are you prepared to receive my line of the prayer?', cid) npcHandler.topic[cid] = 4
            else npcHandler:say('I think you have forgotten to bring your stake, pilgrim.', cid) end
        elseif stakeStorage == 9 then npcHandler:say('You should visit Brewster in Port Hope now.', cid)
        elseif stakeStorage > 9 then npcHandler:say('You already received my line of the prayer.', cid)
        else npcHandler:say('A blessed stake? That is a strange request. Maybe Quentin knows more.', cid) end
        return true
    end

    -- HEALING
    if m == "heal" then
        local healed = false
        if player:getCondition(CONDITION_FIRE) then player:removeCondition(CONDITION_FIRE) healed = true
        elseif player:getCondition(CONDITION_POISON) then player:removeCondition(CONDITION_POISON) healed = true
        elseif player:getCondition(CONDITION_ENERGY) then player:removeCondition(CONDITION_ENERGY) healed = true
        elseif player:getHealth() < 40 then player:addHealth(40 - player:getHealth()) healed = true end
        if healed then player:getPosition():sendMagicEffect(CONST_ME_MAGIC_GREEN) npcHandler:say("There you go.", cid)
        else npcHandler:say("You don't need healing.", cid) end
        return true
    end

    -- CONFIRMATIONS
    if m == "yes" then
        if npcHandler.topic[cid] == 1 then
            local cost = StdModule.calculateRegularBlessingCost(player:getLevel())
            if player:hasBlessing(6) then npcHandler:say("Gods have already blessed you with this blessing!", cid)
            elseif not player:removeTotalMoney(cost) then npcHandler:say("You don't have enough money.", cid)
            else player:addBlessing(6) npcHandler:say("Receive the protection of the twist of fate.", cid) player:getPosition():sendMagicEffect(CONST_ME_MAGIC_BLUE) end
        elseif npcHandler.topic[cid] == 2 then
            player:addItem(18559, 1) player:setStorageValue(Storage.AdventurersGuild.FreeStone.Rahkem, 1) npcHandler:say('Take care.', cid)
        elseif npcHandler.topic[cid] == 3 then
            if player:removeMoneyNpc(30) then player:addItem(18559, 1) npcHandler:say('Take care.', cid)
            else npcHandler:say('You don\'t have enough money.', cid) end
        elseif npcHandler.topic[cid] == 4 then
            if player:getItemCount(5941) > 0 then npcHandler:say('receive my prayer... bring your stake to Brewster in Port Hope.', cid) player:setStorageValue(Storage.FriendsAndTraders.TheBlessedStake, 9) player:getPosition():sendMagicEffect(CONST_ME_MAGIC_BLUE) end
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

npcHandler:setCallback(CALLBACK_MESSAGE_DEFAULT, creatureSayCallback)
npcHandler:addModule(FocusModule:new(), npcHandler.keywordHandler)

-- ANKRAHMUN LORE
keywordHandler:addKeyword({'blessings'}, StdModule.say, {npcHandler = npcHandler, text = 'There are five blessings available... spiritual, phoenix, embrace, suns, solitude.'})

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
