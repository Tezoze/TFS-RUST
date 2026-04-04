-- Trisha - Converted from XML to Lua NpcType
-- Original XML: data/npc/Trisha.xml
-- Original Script: data/npc/scripts/Trisha.lua

local npcName = "Trisha"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a trisha")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(2000)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 142, lookHead = 94, lookBody = 67, lookLegs = 38, lookFeet = 95, lookAddons = 1})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)



-- MASTER SPELL CONFIGURATION
-- Vocations: {4} = Knight, {8} = Elite Knight
local spells = {
    -- HEALING SPELLS
    {name = 'Cure Poison',          price = 150,   level = 10,  vocations = {4, 8}, type = 'healing'},
    {name = 'Wound Cleansing',      price = 0,     level = 8,   vocations = {4, 8}, type = 'healing'},

    -- SUPPORT SPELLS
    {name = 'Find Person',          price = 80,    level = 8,   vocations = {4, 8}, type = 'support'},
    {name = 'Great Light',          price = 500,   level = 13,  vocations = {4, 8}, type = 'support'},
    {name = 'Light',                price = 0,     level = 8,   vocations = {4, 8}, type = 'support'}
}

-- WARRIOR ADDON CONFIG
local addonConfig = {
    ['hardened bones'] = {
        storageValue = 1,
        message = {
            wrongValue = 'Well, I\'ll give you a little hint. They can sometimes be extracted from creatures that consist only of - you guessed it, bones. You need an obsidian knife though.',
            deliver = 'How are you faring with your mission? Have you collected all 100 hardened bones?',
            success = 'I\'m surprised. That\'s pretty good for a man. Now, bring us the 100 turtle shells.'
        },
        itemId = 5925,
        count = 100
    },
    ['turtle shells'] = {
        storageValue = 2,
        message = {
            wrongValue = 'Turtles can be found on some idyllic islands which have recently been discovered.',
            deliver = 'Did you get us 100 turtle shells so we can make new shields?',
            success = 'Well done - for a man. These shells are enough to build many strong new shields. Thank you! Now - show me fighting spirit.'
        },
        itemId = 5899,
        count = 100
    },
    ['fighting spirit'] = {
        storageValue = 3,
        message = {
            wrongValue = 'You should have enough fighting spirit if you are a true hero. Sorry, but you have to figure this one out by yourself. Unless someone grants you a wish.',
            deliver = 'So, can you show me your fighting spirit?',
            success = 'Correct - pretty smart for a man. But the hardest task is yet to come: the claw from a lord among the dragon lords.'
        },
        itemId = 5884,
        count = 1 -- Defaults to 1 if not specified, but good to be explicit
    },
    ['dragon claw'] = {
        storageValue = 4,
        message = {
            wrongValue = 'You cannot get this special red claw from any common dragon in Tibia. It requires a special one, a lord among the lords.',
            deliver = 'Have you actually managed to obtain the dragon claw I asked for?',
            success = 'You did it! I have seldom seen a man as courageous as you. I really have to say that you deserve to wear a spike. Go ask Cornelia to adorn your armour.'
        },
        itemId = 5919,
        count = 1
    }
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
local activeAddonItem = {} -- Tracks which item the player is trying to turn in

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

    -- ==========================================================
    -- WARRIOR OUTFIT QUEST (SHOULDER ADDON)
    -- ==========================================================
    local storage = Storage.OutfitQuest.WarriorShoulderAddon
    
    if m == 'outfit' or m == 'addon' then
        npcHandler:say('Are you talking about my spiky shoulder pad? You can\'t buy one of these. They have to be {earned}.', cid)
        return true
    end

    if m:find('earn') then
        if player:getStorageValue(storage) < 1 then
            npcHandler:say('I\'m not sure if you are enough of a hero to earn them. You could try, though. What do you think?', cid)
            npcHandler.topic[cid] = 1
        elseif player:getStorageValue(storage) >= 1 and player:getStorageValue(storage) < 5 then
            npcHandler:say('Before I can nominate you for an award, please complete your task.', cid)
        elseif player:getStorageValue(storage) == 5 then
            npcHandler:say('You did it! I have seldom seen a man as courageous as you. I really have to say that you deserve to wear a spike. Go ask Cornelia to adorn your armour.', cid)
        end
        return true
    end

    -- Check for quest items (hardened bones, etc.)
    if addonConfig[m] then
        local target = addonConfig[m]
        if player:getStorageValue(storage) ~= target.storageValue then
            npcHandler:say(target.message.wrongValue, cid)
        else
            npcHandler:say(target.message.deliver, cid)
            npcHandler.topic[cid] = 3
            activeAddonItem[cid] = m -- Track the item being discussed
        end
        return true
    end

    -- Quest Confirmations
    if m == 'yes' then
        if npcHandler.topic[cid] == 1 then -- Start Quest
            npcHandler:say({
                'Okay, who knows, maybe you have a chance. A really small one though. Listen up: ...',
                'First, you have to prove your guts by bringing me 100 hardened bones. ...',
                'Next, if you actually managed to collect that many, please complete a small task for our guild and bring us 100 turtle shells. ...',
                'It is said that excellent shields can be created from these. ...',
                'Alright, um, afterwards show me that you have fighting spirit. Any true hero needs plenty of that. ...',
                'The last task is the hardest. You will need to bring me a claw from a mighty dragon king. ...',
                'Did you understand everything I told you and are willing to handle this task?'
            }, cid)
            npcHandler.topic[cid] = 2
            return true
        
        elseif npcHandler.topic[cid] == 2 then -- Confirm Start
            player:setStorageValue(storage, 1)
            player:setStorageValue(Storage.OutfitQuest.DefaultStart, 1)
            npcHandler:say('Excellent! Don\'t forget: Your first task is to bring me 100 hardened bones. Good luck!', cid)
            npcHandler.topic[cid] = 0
            return true

        elseif npcHandler.topic[cid] == 3 then -- Hand in Item
            local itemKey = activeAddonItem[cid]
            if itemKey and addonConfig[itemKey] then
                local target = addonConfig[itemKey]
                local count = target.count or 1
                
                if player:removeItem(target.itemId, count) then
                    player:setStorageValue(storage, player:getStorageValue(storage) + 1)
                    npcHandler:say(target.message.success, cid)
                else
                    npcHandler:say('Why do men always lie?', cid)
                end
            end
            npcHandler.topic[cid] = 0
            activeAddonItem[cid] = nil
            return true
        end
    end

    if m == 'no' and npcHandler.topic[cid] > 0 and npcHandler.topic[cid] ~= 999 then
        if npcHandler.topic[cid] == 1 then
            npcHandler:say('I thought so. Train hard and maybe some day you will be ready to face this mission.', cid)
        elseif npcHandler.topic[cid] == 2 then
            npcHandler:say('Would you like me to repeat the task requirements then?', cid)
            npcHandler.topic[cid] = 1
            return true
        else
            npcHandler:say('Don\'t give up just yet.', cid)
        end
        npcHandler.topic[cid] = 0
        return true
    end

    -- ==========================================================
    -- SPELL TEACHING
    -- ==========================================================
    
    if m == 'spells' then
        if not isKnight(player) then
            npcHandler:say("Sorry, I only teach knights.", cid)
            return true
        end
        npcHandler:say('I can teach you {healing spells} and {support spells}.', cid)
        return true
    end

    if m == 'healing spells' or m == 'healing' or m == 'support spells' or m == 'support' then
        
        if not isKnight(player) then
            npcHandler:say("Sorry, I only teach knights.", cid)
            return true
        end

        local category = 'healing'
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

local function onReleaseFocus(cid)
    activeAddonItem[cid] = nil
    pendingSpell[cid] = nil
end

npcHandler:setMessage(MESSAGE_GREET, 'Salutations, |PLAYERNAME|. What can I do for you?')
npcHandler:setMessage(MESSAGE_FAREWELL, 'Don\'t hurt yourself with that weapon, little one.')
npcHandler:setMessage(MESSAGE_WALKAWAY, 'Be careful on your journeys.')

npcHandler:setCallback(CALLBACK_MESSAGE_DEFAULT, creatureSayCallback)
npcHandler:setCallback(CALLBACK_ONRELEASEFOCUS, onReleaseFocus)

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
