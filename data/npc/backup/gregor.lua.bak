-- Gregor - Converted from XML to Lua NpcType
-- Original XML: data/npc/Gregor.xml
-- Original Script: data/npc/scripts/Gregor.lua

local npcName = "Gregor"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("the first knight")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(2000)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 131, lookHead = 38, lookBody = 38, lookLegs = 38, lookFeet = 38, lookAddons = 3})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- NpcHandler setup
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)

-- Voice module
local voices = { {text = 'Gather around me, young knights! I\'m going to teach you some spells!'} }
npcHandler:addModule(VoiceModule:new(voices))

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

    -- ==========================================================
    -- KNIGHT OUTFIT QUEST (HELMET ADDON)
    -- ==========================================================
    local addonProgress = player:getStorageValue(Storage.OutfitQuest.Knight.AddonHelmet)

    if m:find('task') then
        if not player:isPremium() then
            npcHandler:say('Sorry, but our tasks are only for premium warriors.', cid)
            return true
        end

        if addonProgress < 1 then
            npcHandler:say('You mean you would like to prove that you deserve to wear such a helmet?', cid)
            npcHandler.topic[cid] = 1
        elseif addonProgress == 1 then
            npcHandler:say('Your current task is to bring me 100 perfect behemoth fangs, ' .. player:getName() .. '.', cid)
        elseif addonProgress == 2 then
            npcHandler:say('Your current task is to retrieve the helmet of Ramsay the Reckless from Banuta, ' .. player:getName() .. '.', cid)
        elseif addonProgress == 3 then
            npcHandler:say('Your current task is to obtain a flask of warrior\'s sweat, ' .. player:getName() .. '.', cid)
        elseif addonProgress == 4 then
            npcHandler:say('Your current task is to bring me royal steel, ' .. player:getName() .. '.', cid)
        elseif addonProgress == 5 then
            npcHandler:say('Please talk to Sam and tell him I sent you. I\'m sure he will be glad to refine your helmet, ' .. player:getName() .. '.', cid)
        else
            npcHandler:say('You\'ve already completed the task and can consider yourself a mighty warrior, ' .. player:getName() .. '.', cid)
        end
        return true
    end

    -- Quest Item Inquiries
    if m:find('behemoth fang') then
        if addonProgress == 1 then
            npcHandler:say('Have you really managed to fulfil the task and brought me 100 perfect behemoth fangs?', cid)
            npcHandler.topic[cid] = 3
        else
            npcHandler:say('You\'re not serious asking that, are you? They come from behemoths, of course. Unless there are behemoth rabbits. Duh.', cid)
        end
        return true
    end

    if m:find('ramsay') then
        if addonProgress == 2 then
            npcHandler:say('Did you recover the helmet of Ramsay the Reckless?', cid)
            npcHandler.topic[cid] = 4
        else
            npcHandler:say('These pesky apes steal everything they can get their dirty hands on.', cid)
        end
        return true
    end

    if m:find('sweat') then
        if addonProgress == 3 then
            npcHandler:say('Were you able to get hold of a flask with pure warrior\'s sweat?', cid)
            npcHandler.topic[cid] = 5
        else
            npcHandler:say('Warrior\'s sweat can be magically extracted from headgear worn by a true warrior, but only in small amounts. Djinns are said to be good at magical extractions.', cid)
        end
        return true
    end

    if m:find('royal steel') then
        if addonProgress == 4 then
            npcHandler:say('Ah, have you brought the royal steel?', cid)
            npcHandler.topic[cid] = 6
        else
            npcHandler:say('Royal steel can only be refined by very skilled smiths.', cid)
        end
        return true
    end

    -- Quest Confirmations
    if m == "yes" then
        if npcHandler.topic[cid] == 1 then
            npcHandler:say({
                'Well then, listen closely. First, you will have to prove that you are a fierce and restless warrior by bringing me 100 perfect behemoth fangs. ...',
                'Secondly, please retrieve a helmet for us which has been lost a long time ago. The famous Ramsay the Reckless wore it when exploring an ape settlement. ...',
                'Third, we need a new flask of warrior\'s sweat. We\'ve run out of it recently, but we need a small amount for the show battles in our arena. ...',
                'Lastly, I will have our smith refine your helmet if you bring me royal steel, an especially noble metal. ...',
                'Did you understand everything I told you and are willing to handle this task?'
            }, cid)
            npcHandler.topic[cid] = 2
            return true
        
        elseif npcHandler.topic[cid] == 2 then
            player:setStorageValue(Storage.OutfitQuest.Ref, math.max(0, player:getStorageValue(Storage.OutfitQuest.Ref)) + 1)
            player:setStorageValue(Storage.OutfitQuest.Knight.AddonHelmet, 1)
            player:setStorageValue(Storage.OutfitQuest.Knight.MissionHelmet, 1)
            npcHandler:say('Alright then. Come back to me once you have collected 100 perfect behemoth fangs.', cid)
            npcHandler.topic[cid] = 0
            return true

        elseif npcHandler.topic[cid] == 3 then
            if not player:removeItem(5893, 100) then
                npcHandler:say('Lying is not exactly honourable, ' .. player:getName() .. '. Shame on you.', cid)
                return true
            end
            player:setStorageValue(Storage.OutfitQuest.Knight.AddonHelmet, 2)
            player:setStorageValue(Storage.OutfitQuest.Knight.MissionHelmet, 2)
            player:setStorageValue(Storage.OutfitQuest.Knight.RamsaysHelmetDoor, 1)
            npcHandler:say('I\'m deeply impressed, brave Knight ' .. player:getName() .. '. I expected nothing less from you. Now, please retrieve Ramsay\'s helmet.', cid)
            npcHandler.topic[cid] = 0
            return true

        elseif npcHandler.topic[cid] == 4 then
            if not player:removeItem(5924, 1) then
                npcHandler:say('Lying is not exactly honourable, ' .. player:getName() .. '. Shame on you.', cid)
                return true
            end
            player:setStorageValue(Storage.OutfitQuest.Knight.AddonHelmet, 3)
            player:setStorageValue(Storage.OutfitQuest.Knight.MissionHelmet, 3)
            npcHandler:say('Good work, brave Knight ' .. player:getName() .. '! Even though it is damaged, it has a lot of sentimental value. Now, please bring me warrior\'s sweat.', cid)
            npcHandler.topic[cid] = 0
            return true

        elseif npcHandler.topic[cid] == 5 then
            if not player:removeItem(5885, 1) then
                npcHandler:say('Lying is not exactly honourable, ' .. player:getName() .. '. Shame on you.', cid)
                return true
            end
            player:setStorageValue(Storage.OutfitQuest.Knight.AddonHelmet, 4)
            player:setStorageValue(Storage.OutfitQuest.Knight.MissionHelmet, 4)
            npcHandler:say('Now that is a pleasant surprise, brave Knight ' .. player:getName() .. '! There is only one task left now: Obtain royal steel to have your helmet refined.', cid)
            npcHandler.topic[cid] = 0
            return true

        elseif npcHandler.topic[cid] == 6 then
            if not player:removeItem(5887, 1) then
                npcHandler:say('Lying is not exactly honourable, ' .. player:getName() .. '. Shame on you.', cid)
                return true
            end
            player:setStorageValue(Storage.OutfitQuest.Knight.AddonHelmet, 5)
            player:setStorageValue(Storage.OutfitQuest.Knight.MissionHelmet, 5)
            npcHandler:say('You truly deserve to wear an adorned helmet, brave Knight ' .. player:getName() .. '. Please talk to Sam and tell him I sent you. I\'m sure he will be glad to refine your helmet.', cid)
            npcHandler.topic[cid] = 0
            return true
        end
    end

    if m == "no" then
        if npcHandler.topic[cid] == 1 then
            npcHandler:say('Bah. Then you will have to wait for the day these helmets are sold in shops, but that will not happen before hell freezes over.', cid)
            npcHandler.topic[cid] = 0
        elseif npcHandler.topic[cid] == 2 then
            npcHandler:say('Would you like me to repeat the task requirements then?', cid)
            npcHandler.topic[cid] = 1
        elseif npcHandler.topic[cid] >= 3 then
            npcHandler:say('There is no need to rush anyway.', cid)
            npcHandler.topic[cid] = 0
        end
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

npcHandler:setMessage(MESSAGE_GREET, "Greetings, |PLAYERNAME|. What do you want?")
npcHandler:setMessage(MESSAGE_FAREWELL, "Be careful on your journeys.")
npcHandler:setMessage(MESSAGE_WALKAWAY, "Be careful on your journeys.")

npcHandler:setCallback(CALLBACK_MESSAGE_DEFAULT, creatureSayCallback)

-- FLAVOR KEYWORDS
keywordHandler:addKeyword({'job'}, StdModule.say, {npcHandler = npcHandler, text = "I am the first knight. I trained some of the greatest heroes of Tibia."})
keywordHandler:addKeyword({'heroes'}, StdModule.say, {npcHandler = npcHandler, text = "Of course, you heard of them. Knights are the best fighters in Tibia."})
keywordHandler:addKeyword({'king'}, StdModule.say, {npcHandler = npcHandler, text = "Hail to our King!"})
keywordHandler:addKeyword({'name'}, StdModule.say, {npcHandler = npcHandler, text = "You are joking, eh? Of course, you know me. I am Gregor, the first knight."})
keywordHandler:addKeyword({'gregor'}, StdModule.say, {npcHandler = npcHandler, text = "A great name, isn't it?"})
keywordHandler:addKeyword({'tibia'}, StdModule.say, {npcHandler = npcHandler, text = "Beautiful Tibia. And with our help everyone is save."})
keywordHandler:addKeyword({'time'}, StdModule.say, {npcHandler = npcHandler, text = "It is time to join the Knights!"})
keywordHandler:addKeyword({'knights'}, StdModule.say, {npcHandler = npcHandler, text = "Knights are the warriors of Tibia. Without us, no one would be safe. Every brave and strong man or woman can join us."})
keywordHandler:addKeyword({'bozo'}, StdModule.say, {npcHandler = npcHandler, text = "Some day someone will make something happen to him..."})
keywordHandler:addKeyword({'elane'}, StdModule.say, {npcHandler = npcHandler, text = "A bow might be a fine weapon for someone not strong enough to wield a REAL weapon."})
keywordHandler:addKeyword({'frodo'}, StdModule.say, {npcHandler = npcHandler, text = "I and my students often share a cask of beer or wine at Frodo's hut."})
keywordHandler:addKeyword({'gorn'}, StdModule.say, {npcHandler = npcHandler, text = "Always concerned with his profit. What a loss! He was adventuring with baxter in the old days."})
keywordHandler:addKeyword({'baxter'}, StdModule.say, {npcHandler = npcHandler, text = "He was an adventurer once."})
keywordHandler:addKeyword({'lynda'}, StdModule.say, {npcHandler = npcHandler, text = "Before she became a priest she won the Miss Tibia contest three times in a row."})
keywordHandler:addKeyword({'mcronald'}, StdModule.say, {npcHandler = npcHandler, text = "Peaceful farmers."})
keywordHandler:addKeyword({'ferumbras'}, StdModule.say, {npcHandler = npcHandler, text = "A fine game to hunt. But be careful, he cheats!"})
keywordHandler:addKeyword({'muriel'}, StdModule.say, {npcHandler = npcHandler, text = "Bah, go away with these sorcerer tricks. Only cowards use tricks."})
keywordHandler:addKeyword({'oswald'}, StdModule.say, {npcHandler = npcHandler, text = "What an idiot."})
keywordHandler:addKeyword({'quentin'}, StdModule.say, {npcHandler = npcHandler, text = "I will never understand this peaceful monks and priests."})
keywordHandler:addKeyword({'sam'}, StdModule.say, {npcHandler = npcHandler, text = "He has the muscles, but lacks the guts."})
keywordHandler:addKeyword({'tibianus'}, StdModule.say, {npcHandler = npcHandler, text = "Hail to our King!"})
keywordHandler:addKeyword({'outfit'}, StdModule.say, {npcHandler = npcHandler, text = "Only the bravest warriors may wear adorned helmets. They are traditionally awarded after having completed a difficult task for our guild."})
keywordHandler:addKeyword({'helmet'}, StdModule.say, {npcHandler = npcHandler, text = "Only the bravest warriors may wear adorned helmets. They are traditionally awarded after having completed a difficult task for our guild."})

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