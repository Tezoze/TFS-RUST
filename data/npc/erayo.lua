-- Erayo - Converted from XML to Lua NpcType
-- Original XML: data/npc/Erayo.xml
-- Original Script: data/npc/scripts/Erayo.lua

local npcName = "Erayo"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a erayo")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(2000)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 152, lookHead = 86, lookBody = 125, lookLegs = 86, lookFeet = 87, lookAddons = 3})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)


-- 1. VOICE MODULE
local voices = {
    { text = 'What was that noise?' },
    { text = 'I am one with the shadows.' },
    { text = 'Sneaking is an art.' }
}
npcHandler:addModule(VoiceModule:new(voices))

-- 2. QUEST CONFIGURATION
local STORAGE_ADDON = Storage.OutfitQuest.AssassinFirstAddon
local ASSASSIN_MALE = 152
local ASSASSIN_FEMALE = 156

local questConfig = {
    ['blue cloth'] = {
        storageValue = 1, 
        itemId = 5912, 
        count = 50,
        text = {
            confirm = 'Brought the 50 pieces of blue cloth?', 
            success = 'Good. Get me 50 pieces of green cloth now.'
        }
    },
    ['green cloth'] = {
        storageValue = 2, 
        itemId = 5910, 
        count = 50,
        text = {
            confirm = 'Brought the 50 pieces of green cloth?', 
            success = 'Good. Get me 50 pieces of red cloth now.'
        }
    },
    ['red cloth'] = {
        storageValue = 3, 
        itemId = 5911, 
        count = 50,
        text = {
            confirm = 'Brought the 50 pieces of red cloth?', 
            success = 'Good. Get me 50 pieces of brown cloth now.'
        }
    },
    ['brown cloth'] = {
        storageValue = 4, 
        itemId = 5913, 
        count = 50,
        text = {
            confirm = 'Brought the 50 pieces of brown cloth?', 
            success = 'Good. Get me 50 pieces of yellow cloth now.'
        }
    },
    ['yellow cloth'] = {
        storageValue = 5, 
        itemId = 5914, 
        count = 50,
        text = {
            confirm = 'Brought the 50 pieces of yellow cloth?', 
            success = 'Good. Get me 50 pieces of white cloth now.'
        }
    },
    ['white cloth'] = {
        storageValue = 6, 
        itemId = 5909, 
        count = 50,
        text = {
            confirm = 'Brought the 50 pieces of white cloth?', 
            success = 'Good. Get me 10 spools of yarn now.'
        }
    },
    ['spools of yarn'] = {
        storageValue = 7, 
        itemId = 5886, 
        count = 10,
        text = {
            confirm = 'Brought the 10 spools of yarn?', 
            success = 'Thanks. That\'s it, you\'re done. Good job, |PLAYERNAME|. I keep my promise. Here\'s my old assassin head piece.'
        }
    }
}

-- Aliases to map inputs to config keys
local itemAliases = {
    ['yarn'] = 'spools of yarn'
}

-- Helper function to find what item is needed based on storage
local function getRequiredItemName(storage)
    for name, config in pairs(questConfig) do
        if config.storageValue == storage then
            return name
        end
    end
    return nil
end

-- State variable for current transaction
local pendingItem = {}

local function creatureSayCallback(cid, type, msg)
    if not npcHandler:isFocused(cid) then
        return false
    end

    local player = Player(cid)
    local m = msg:lower()

    -- 1. QUEST START
    if m:find('addon') or m:find('outfit') or m:find('head') then
        -- Must have the base outfit (Male or Female)
        if player:hasOutfit(ASSASSIN_MALE) or player:hasOutfit(ASSASSIN_FEMALE) then
            local currentStage = player:getStorageValue(STORAGE_ADDON)
            
            if currentStage < 1 then
                npcHandler:say('Vescu gave you an assassin outfit? Haha. Noticed it lacks the head piece? You look a bit silly. Want my old head piece?', cid)
                npcHandler.topic[cid] = 1
            elseif currentStage >= 1 and currentStage < 8 then
                -- INTELLIGENT RESPONSE: Tells you exactly what he wants
                local needed = getRequiredItemName(currentStage)
                if needed then
                    npcHandler:say('I am still waiting for the materials I asked for: {' .. needed .. '}. Bring them to me.', cid)
                else
                    npcHandler:say('I am waiting for materials.', cid)
                end
            else
                npcHandler:say('You already have my head piece.', cid)
            end
        else
            npcHandler:say('You don\'t look like an assassin to me. Get the outfit first.', cid)
        end
        return true
    end

    -- 2. ITEM HAND-IN CHECK
    local key = itemAliases[m] or m
    
    if questConfig[key] and npcHandler.topic[cid] == 0 then
        local cfg = questConfig[key]
        if player:getStorageValue(STORAGE_ADDON) == cfg.storageValue then
            npcHandler:say(cfg.text.confirm, cid)
            npcHandler.topic[cid] = 3
            pendingItem[cid] = key
        end
        return true
    end

    -- 3. CONFIRMATIONS
    if m == 'yes' then
        -- Accept Quest
        if npcHandler.topic[cid] == 1 then
            npcHandler:say({
                'Thought so. Could use some help anyway. Listen, I need stuff. Someone gave me a strange assignment - sneak into Thais castle at night and shroud it with cloth without anyone noticing it. ...',
                'I wonder why anyone would want to shroud a castle, but as long as the guy pays, no problem, I\'ll do the sneaking part. Need a lot of cloth though. ...',
                'Gonna make it colourful. Bring me 50 pieces of {blue cloth}, 50 pieces of {green cloth}, 50 pieces of {red cloth}, 50 pieces of {brown cloth}, 50 pieces of {yellow cloth} and 50 pieces of {white cloth}. ...',
                'Besides, gonna need 10 {spools of yarn}. Understood?'
            }, cid)
            npcHandler.topic[cid] = 2
        
        -- Confirm Start
        elseif npcHandler.topic[cid] == 2 then
            if player:getStorageValue(Storage.OutfitQuest.DefaultStart) ~= 1 then
                player:setStorageValue(Storage.OutfitQuest.DefaultStart, 1)
            end
            player:setStorageValue(STORAGE_ADDON, 1)
            npcHandler:say('Good. Start with the {blue cloth}. I\'ll wait.', cid)
            npcHandler.topic[cid] = 0
        
        -- Hand in Item
        elseif npcHandler.topic[cid] == 3 then
            local key = pendingItem[cid]
            if key and questConfig[key] then
                local cfg = questConfig[key]
                if player:removeItem(cfg.itemId, cfg.count) then
                    
                    local newStorage = player:getStorageValue(STORAGE_ADDON) + 1
                    player:setStorageValue(STORAGE_ADDON, newStorage)
                    
                    -- Check if this was the last item (Yarn)
                    if newStorage == 8 then
                        player:addOutfitAddon(ASSASSIN_MALE, 1)
                        player:addOutfitAddon(ASSASSIN_FEMALE, 1)
                        player:getPosition():sendMagicEffect(CONST_ME_MAGIC_BLUE)
                    end
                    
                    npcHandler:say(cfg.text.success, cid)
                else
                    npcHandler:say('You don\'t have the required items.', cid)
                end
            end
            npcHandler.topic[cid] = 0
            pendingItem[cid] = nil
        end
        return true
    end

    if m == 'no' and npcHandler.topic[cid] > 0 then
        npcHandler:say('Maybe another time.', cid)
        npcHandler.topic[cid] = 0
        return true
    end

    return true
end

local function onReleaseFocus(cid)
    pendingItem[cid] = nil
end

npcHandler:setMessage(MESSAGE_GREET, 'What the... I mean, of course I sensed you.')

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
