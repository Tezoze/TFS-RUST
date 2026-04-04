-- Vescu - Converted from XML to Lua NpcType
-- Original XML: data/npc/Vescu.xml
-- Original Script: data/npc/scripts/Vescu.lua

local npcName = "Vescu"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a vescu")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(2000)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 152, lookHead = 119, lookBody = 120, lookLegs = 119, lookFeet = 101, lookAddons = 3})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)


local voices = {
    { text = '<hicks>' },
    { text = 'I\'m the b-best k-killer around.' },
    { text = 'Where did I put my b-booze?' },
    { text = 'Uhh... catching f-flies... <burp>' }
}
npcHandler:addModule(VoiceModule:new(voices))

-- QUEST ITEM CONFIGURATION
-- Note: count = 1 is default if not specified
local questConfig = {
    ['bonelord eyes'] = {
        storageValue = 1,
        itemId = 5898,
        count = 30,
        text = {
            confirm = 'Have you really managed to bring me 30 bonelord eyes? <hicks>',
            incomplete = 'Do bonelord eyes continue blinking when they are seperated from the bonelord? That is a scary thought.',
            success = 'Aw-awsome! <hicks> Squishy! Now, please bring me 10 red dragon scales.'
        }
    },
    ['red dragon scales'] = {
        storageValue = 2,
        itemId = 5882,
        count = 10,
        text = {
            confirm = 'D-did you get all of the 10 red dragon scales? <hicks>',
            incomplete = 'Have you ever wondered if a red dragon means \'stop\' whereas a green dragon means \'go\'?',
            success = 'G-good work, ... wha-what\'s your name again? <hicks> Anyway... come back with 30 lizard scales.'
        }
    },
    ['lizard scales'] = {
        storageValue = 3,
        itemId = 5881,
        count = 30,
        text = {
            confirm = 'Ah, are those - <hicks> - the 30 lizard scales?',
            incomplete = 'I once had a girlfriend c-called L-lizzie. She had s-scales too.',
            success = 'This potion will become p-pretty scaly. I\'m not sure yet if I want to d-drink that. I think the 20 fish fins which come next won\'t really improve it. <hicks>'
        }
    },
    ['fish fins'] = {
        storageValue = 4,
        itemId = 5895,
        count = 20,
        text = {
            confirm = 'Eww, is that disgusting smell coming from the 20 fish fins? <burps>',
            incomplete = 'Not normal fish fins of course. We need <hicks> Quara fish fins. If you haven\'t h-heard about them, ask the - <hicks> - plorer society.',
            success = 'Alrrrrrrright! Thanks for the f-fish. Get me the 20 ounces of vampire dust now. I\'ll have another b-beer.'
        }
    },
    ['vampire dust'] = {
        storageValue = 5,
        itemId = 5905,
        count = 20,
        text = {
            confirm = 'Have you collected 20 ounces of vampire d-dust? <hicks>',
            incomplete = 'Don\'t you think vampires have something - <hicks> - romantic about them? I think you need a b-blessed steak though to turn them into d-dust.',
            success = 'Tha-thank you. Trolls are good for something a-after all. Bring me the 10 ounces of demon dust now. <hicks>'
        }
    },
    ['demon dust'] = {
        storageValue = 6,
        itemId = 5906,
        count = 10,
        text = {
            confirm = 'Have you slain enough d-demons to gather 10 ounces of demon dust? <hicks>',
            incomplete = 'I like d-demons. They are just as pretty as flamingos. But you need a blessed stake or something to get demon dust. <hicks>',
            success = 'G-great. You\'re a reeeal k-killer like me, eh? I think I\'ll g-give you something fun when the potion is complete. But first, b-bring me warrior\'s sweat.'
        }
    },
    ['warrior\'s sweat'] = {
        storageValue = 7,
        itemId = 5885,
        count = 1,
        text = {
            confirm = 'This s-smells even worse than the fish fins. Is that warrior\'s sweat?',
            incomplete = 'If you can\'t sweat enough yourself, go ask a Djinn. They do - <hicks> magical <hicks> - tractions. Err, extractions.',
            success = 'Yahaha! Here we g-go. I\'ll just take a small sip - <gulp>. Okay, this is disgusting, but it seems to work. I\'ll teach you something fun, remind me to tell you a secret sometime.'
        }
    }
}

-- Aliases for keywords to map to the config table keys
local itemAliases = {
    ['bonelord eye'] = 'bonelord eyes',
    ['red dragon scale'] = 'red dragon scales',
    ['lizard scale'] = 'lizard scales',
    ['fish fin'] = 'fish fins',
    ['vampire dust'] = 'vampire dust',
    ['demon dust'] = 'demon dust',
    ['warrior\'s sweat'] = 'warrior\'s sweat'
}

-- State tracking
local pendingItem = {}

local function greetCallback(cid)
    local player = Player(cid)
    -- NPC only responds if the player is drunk (Condition Drunk)
    if player:hasCondition(CONDITION_DRUNK) then
        npcHandler:setMessage(MESSAGE_GREET, 'Hey t-there, you look like someone who enjoys a good booze.')
        return true
    else
        npcHandler:say('Oh, two t-trolls. Hellooo, wittle twolls. <hicks>', cid)
        return false
    end
end

local function creatureSayCallback(cid, type, msg)
    if not npcHandler:isFocused(cid) then return false end

    local player = Player(cid)
    local m = msg:lower()
    local storage = Storage.OutfitQuest.AssassinBaseOutfit

    -- 1. STARTING THE QUEST LOGIC
    if m:find('sober') then
        npcHandler:say('I wish there was like a potion which makes you sober in an instant. Dwarven rings wear off so fast. <hicks>', cid)
        return true
    end

    if m:find('potion') then
        if player:getStorageValue(storage) < 1 then
            npcHandler:say('It\'s so hard to know the exact time when to stop drinking. <hicks> C-could you help me to brew such a potion?', cid)
            npcHandler.topic[cid] = 1
        end
        return true
    end

    -- 2. FINAL REWARD (SECRET)
    if m:find('secret') then
        if player:getStorageValue(storage) == 8 then
            npcHandler:say('Right. <hicks> Since you helped me to b-brew that potion and thus ensured the high quality of my work <hicks>, I\'ll give you my old assassin costume. It lacks the head part, but it\'s almost like new. Don\'t pretend to be me though, \'kay? <hicks>', cid)
            player:addOutfit(156) -- Male Assassin
            player:addOutfit(152) -- Female Assassin
            player:getPosition():sendMagicEffect(CONST_ME_MAGIC_GREEN)
            player:setStorageValue(storage, 9)
        end
        return true
    end

    -- 3. ITEM HAND-IN CHECK
    -- Resolve the alias (e.g., "bonelord eye" -> "bonelord eyes")
    local key = itemAliases[m] or m
    
    if questConfig[key] and npcHandler.topic[cid] == 0 then
        local cfg = questConfig[key]
        if player:getStorageValue(storage) == cfg.storageValue then
            npcHandler:say(cfg.text.confirm, cid)
            npcHandler.topic[cid] = 3
            pendingItem[cid] = key
        else
            npcHandler:say(cfg.text.incomplete, cid)
        end
        return true
    end

    -- 4. CONFIRMATIONS (YES/NO)
    if m == 'yes' then
        -- Topic 1: Explaining the ingredients
        if npcHandler.topic[cid] == 1 then
            npcHandler:say({
                'You\'re a true buddy. I promise I will t-try to avoid killing you even if someone asks me to. <hicks> ...',
                'Listen, I have this old formula from my grandma. <hicks> It says... 30 bonelord eyes... 10 red dragon scales. ...',
                'Then 30 lizard scales... 20 fish fins - ew, this sounds disgusting, I wonder if this is really a potion or rather a cleaning agent. ...',
                'Add 20 ounces of vampire dust, 10 ounces of demon dust and mix well with one flask of warrior\'s sweat. <hicks> ...',
                'Okayyy, this is a lot... we\'ll take this step by step. <hicks> Will you help me gathering 30 bonelord eyes?'
            }, cid)
            npcHandler.topic[cid] = 2
        
        -- Topic 2: Confirming start of quest
        elseif npcHandler.topic[cid] == 2 then
            if player:getStorageValue(Storage.OutfitQuest.DefaultStart) ~= 1 then
                player:setStorageValue(Storage.OutfitQuest.DefaultStart, 1)
            end
            player:setStorageValue(storage, 1)
            npcHandler:say('G-good. Go get them, I\'ll have a beer in the meantime.', cid)
            npcHandler.topic[cid] = 0
        
        -- Topic 3: Handing in an item
        elseif npcHandler.topic[cid] == 3 then
            local key = pendingItem[cid]
            if key and questConfig[key] then
                local cfg = questConfig[key]
                if player:removeItem(cfg.itemId, cfg.count) then
                    player:setStorageValue(storage, player:getStorageValue(storage) + 1)
                    npcHandler:say(cfg.text.success, cid)
                else
                    npcHandler:say('Next time you lie to me I\'ll k-kill you. <hicks> Don\'t think I can\'t aim well just because I\'m d-drunk.', cid)
                end
            end
            npcHandler.topic[cid] = 0
            pendingItem[cid] = nil
        end
        return true
    end

    if m == 'no' then
        if npcHandler.topic[cid] == 3 then
            npcHandler:say('H-hurry up! <hicks> I have to start working soon.', cid)
        else
            npcHandler:say('Then not <hicks>.', cid)
        end
        npcHandler.topic[cid] = 0
        return true
    end

    return true
end

local function onReleaseFocus(cid)
    pendingItem[cid] = nil
end

npcHandler:setMessage(MESSAGE_FAREWELL, 'T-time for another b-beer. <hicks>')
npcHandler:setMessage(MESSAGE_WALKAWAY, 'Oh, two t-trolls. Hellooo, wittle twolls. <hicks>')

npcHandler:setCallback(CALLBACK_GREET, greetCallback)
npcHandler:setCallback(CALLBACK_MESSAGE_DEFAULT, creatureSayCallback)
npcHandler:setCallback(CALLBACK_ONRELEASEFOCUS, onReleaseFocus)


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
