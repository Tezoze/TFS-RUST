-- Ishebad - Converted from XML to Lua NpcType
-- Original XML: data/npc/Ishebad.xml
-- Original Script: data/npc/scripts/Ishebad.lua

local npcName = "Ishebad"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a ishebad")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(2000)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 65})
npcType:speechBubble(SPEECHBUBBLE_NORMAL)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)


local function creatureSayCallback(cid, type, msg)
    if not npcHandler:isFocused(cid) then
        return false
    end

    local player = Player(cid)
    if not player then
        return false
    end
    
    local m = msg:lower()

    -- Promotion System
    if m:find("promot") then
        if player:getStorageValue(PlayerStorageKeys.promotion) == 1 then
            npcHandler:say("You are already promoted.", cid)
        elseif player:getLevel() < 20 then
            npcHandler:say("You need to be at least level 20 to be promoted.", cid)
        else
            npcHandler:say("Do you want to be promoted in your vocation for 20000 gold?", cid)
            npcHandler.topic[cid] = 1
        end
        return true
    end

    if m == "yes" and npcHandler.topic[cid] == 1 then
        if player:getStorageValue(PlayerStorageKeys.promotion) == 1 then
            npcHandler:say("You are already promoted.", cid)
        elseif not player:removeTotalMoney(20000) then
            npcHandler:say("You do not have enough money.", cid)
        else
            local promotion = player:getVocation():getPromotion()
            player:setVocation(promotion)
            player:setStorageValue(PlayerStorageKeys.promotion, 1)
            npcHandler:say("Congratulations! You are now promoted. Go forth and conquer!", cid)
        end
        npcHandler.topic[cid] = 0
        return true
    end

    if m == "no" and npcHandler.topic[cid] == 1 then
        npcHandler:say("Ok, whatever.", cid)
        npcHandler.topic[cid] = 0
        return true
    end

    return true
end

local function greetCallback(cid)
    npcHandler.topic[cid] = 0
    return true
end

npcHandler:setCallback(CALLBACK_GREET, greetCallback)
npcHandler:setCallback(CALLBACK_MESSAGE_DEFAULT, creatureSayCallback)

npcHandler:setMessage(MESSAGE_GREET, 'Be mourned, pilgrim in flesh. Are you looking for a {promotion}?')
npcHandler:setMessage(MESSAGE_FAREWELL, 'Good bye, |PLAYERNAME|!')
npcHandler:setMessage(MESSAGE_WALKAWAY, 'Good bye, |PLAYERNAME|!')


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
