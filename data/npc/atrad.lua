-- Atrad - Converted from XML to Lua NpcType
-- Original XML: data/npc/Atrad.xml
-- Original Script: data/npc/scripts/Atrad.lua

local npcName = "Atrad"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a atrad")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(2000)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 152, lookHead = 77, lookBody = 113, lookLegs = 132, lookFeet = 94, lookAddons = 3})
npcType:speechBubble(SPEECHBUBBLE_4)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)

local shopModule = ShopModule:new()
npcHandler:addModule(shopModule)

-- CONFIG
local STORAGE_KATANA = Storage.OutfitQuest.Assassin.SecondAddon
local ASSASSIN_MALE = 152
local ASSASSIN_FEMALE = 156
local ITEM_NOSE_RING = 5804
local ITEM_BEHEMOTH_CLAW = 5930
local ITEM_ASSASSIN_STAR = 7368


-- 1. GREETING CONDITION (Assassin Outfit + Fire)
local function greetCallback(cid)
    local player = Player(cid)
    
    -- Check for Assassin Base Outfit
    if not (player:hasOutfit(ASSASSIN_MALE) or player:hasOutfit(ASSASSIN_FEMALE)) then
        npcHandler:say("I don't talk to amateurs. Get lost.", cid)
        return false
    end

    -- Check for Fire Condition
    if not player:getCondition(CONDITION_FIRE) then
        npcHandler:say("You are not tempered enough. Come back when you are burning with determination.", cid)
        return false
    end
    
    npcHandler:setMessage(MESSAGE_GREET, "You look hot, |PLAYERNAME|. What do you want?")
    return true
end

-- 2. TRADE BLOCKER (Level 80+)
local function onTradeRequest(cid)
    local player = Player(cid)
    if player:getLevel() < 80 then
        npcHandler:say("You are not experienced enough to trade with me. Come back when you are level 80.", cid)
        return false
    end
    return true
end

-- 3. DIALOGUE / QUEST LOGIC
local function creatureSayCallback(cid, type, msg)
    if not npcHandler:isFocused(cid) then
        return false
    end

    local player = Player(cid)
    local m = msg:lower()

    -- ADDON QUEST (Katana)
    if m == "addon" or m == "outfit" or m == "katana" then
        if player:getStorageValue(STORAGE_KATANA) < 1 then
            npcHandler:say("You managed to deceive Erayo? Impressive. Well, I guess, since you have come that far, I might as well give you a task too, eh?", cid)
            npcHandler.topic[cid] = 1
        elseif player:getStorageValue(STORAGE_KATANA) == 1 then
            npcHandler:say("Did you bring the stuff? A {nose ring} and a {behemoth claw}?", cid)
            npcHandler.topic[cid] = 2
        else
            npcHandler:say("You already have my katana.", cid)
        end
        return true
    end

    -- CONFIRM QUEST START
    if m == "yes" and npcHandler.topic[cid] == 1 then
        npcHandler:say("Okay, listen up. I don't have a list of stupid objects, I just want two things. A {behemoth claw} and a {nose ring}. Got that?", cid)
        npcHandler.topic[cid] = 3
        return true
    
    elseif m == "yes" and npcHandler.topic[cid] == 3 then
        npcHandler:say("Good. Come back when you have BOTH. Should be clear where to get a behemoth claw from. There's a horned fox who wears a nose ring. Good luck.", cid)
        player:setStorageValue(STORAGE_KATANA, 1)
        npcHandler.topic[cid] = 0
        return true
    end

    -- HAND IN QUEST ITEMS
    if (m == "nose ring" or m == "ring" or m == "claw" or m == "yes") and npcHandler.topic[cid] == 2 then
        if player:getItemCount(ITEM_NOSE_RING) >= 1 and player:getItemCount(ITEM_BEHEMOTH_CLAW) >= 1 then
            if player:removeItem(ITEM_NOSE_RING, 1) and player:removeItem(ITEM_BEHEMOTH_CLAW, 1) then
                npcHandler:say("I see you brought my stuff. Good. I'll keep my promise: Here's the katana in return.", cid)
                
                player:addOutfitAddon(ASSASSIN_MALE, 2)
                player:addOutfitAddon(ASSASSIN_FEMALE, 2)
                player:getPosition():sendMagicEffect(CONST_ME_MAGIC_BLUE)
                
                player:setStorageValue(STORAGE_KATANA, 2)
                npcHandler.topic[cid] = 0
            end
        else
            npcHandler:say("You don't have the nose ring and the behemoth claw.", cid)
            npcHandler.topic[cid] = 0
        end
        return true
    end

    return true
end

-- SHOP CONFIGURATION
-- We set the 4th argument (subtype) to 0.
-- Arguments: {Names}, ItemID, Cost, SubType, RealName
shopModule:addBuyableItem({'assassin star'}, ITEM_ASSASSIN_STAR, 100, 0, 'assassin star')

-- Register Callbacks
npcHandler:setCallback(CALLBACK_GREET, greetCallback)
npcHandler:setCallback(CALLBACK_ONTRADEREQUEST, onTradeRequest)
npcHandler:setCallback(CALLBACK_MESSAGE_DEFAULT, creatureSayCallback)


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
