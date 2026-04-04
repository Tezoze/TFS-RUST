-- Test NPC defined purely in Lua using NpcType
-- This NPC demonstrates the new full Lua NPC system

local npcName = "Test Lua NPC"
local npcType = Game.createNpcType(npcName)

-- Set basic properties
npcType:name(npcName)
npcType:nameDescription("a test NPC")
npcType:health(100)
npcType:maxHealth(100)

-- Movement settings
npcType:walkInterval(2000)
npcType:walkRadius(3)
npcType:baseSpeed(100)

-- Appearance
npcType:outfit({
    lookType = 128, -- Citizen male
    lookHead = 78,
    lookBody = 114,
    lookLegs = 88,
    lookFeet = 115
})

-- Light (color, level) - no light
npcType:light(0, 0)

-- Properties
npcType:speechBubble(SPEECHBUBBLE_NORMAL)
npcType:isPushable(false)
npcType:floorChange(false)
npcType:canPushItems(false)
npcType:canPushCreatures(false)

-- Voice system - random messages (public)
npcType:addVoice("Hello there, traveler!", 60000, 5, false)
npcType:addVoice("I am a Lua-defined NPC!", 60000, 5, false)

-- Track focused player
local focusedPlayer = nil
local lastTalkTime = 0

-- Helper function to say to player via NPC channel
local function sayToPlayer(npc, player, message)
    npc:say(message, TALKTYPE_PRIVATE_NP, false, player, npc:getPosition())
end

-- Event callbacks
npcType:eventType(NPCS_EVENT_SAY)
npcType:onSay(function(npc, creature, type, message)
    if not creature:isPlayer() then
        return
    end
    
    local player = creature:getPlayer()
    local lowerMessage = message:lower()
    
    if lowerMessage == "hi" or lowerMessage == "hello" then
        if focusedPlayer and focusedPlayer == player:getId() then
            sayToPlayer(npc, player, "I'm already talking to you!")
        else
            focusedPlayer = player:getId()
            lastTalkTime = os.time()
            npc:setFocus(player)
            sayToPlayer(npc, player, "Hello " .. player:getName() .. "! I am a Lua-defined NPC! Say {test}, {features}, or {bye}.")
        end
        return
    end
    
    -- Only respond if focused
    if focusedPlayer ~= player:getId() then
        return
    end
    
    lastTalkTime = os.time()
    npc:setFocus(player)
    
    if lowerMessage == "bye" then
        sayToPlayer(npc, player, "Goodbye, " .. player:getName() .. "!")
        focusedPlayer = nil
        npc:setFocus(nil)
        
    elseif lowerMessage == "test" then
        sayToPlayer(npc, player, "This NPC was created using Game.createNpcType() - no XML required!")
        
    elseif lowerMessage == "features" then
        sayToPlayer(npc, player, "I support: outfit, health, walking, speech bubble, light, voices, shop items, and all event callbacks!")
        
    elseif lowerMessage == "job" then
        sayToPlayer(npc, player, "I am a test NPC to demonstrate the Lua NPC system.")
    end
end)

npcType:eventType(NPCS_EVENT_APPEAR)
npcType:onAppear(function(npc, creature)
    -- When a player appears nearby
end)

npcType:eventType(NPCS_EVENT_DISAPPEAR)
npcType:onDisappear(function(npc, creature)
    if creature:isPlayer() and focusedPlayer == creature:getId() then
        focusedPlayer = nil
        npc:setFocus(nil)
    end
end)

npcType:eventType(NPCS_EVENT_THINK)
npcType:onThink(function(npc, interval)
    -- Idle timeout (2 minutes)
    if focusedPlayer and os.time() - lastTalkTime > 120 then
        local player = Player(focusedPlayer)
        if player then
            sayToPlayer(npc, player, "Goodbye.")
        end
        focusedPlayer = nil
        npc:setFocus(nil)
    end
end)

npcType:eventType(NPCS_EVENT_CLOSECHANNEL)
npcType:onCloseChannel(function(npc, creature)
    if creature:isPlayer() and focusedPlayer == creature:getId() then
        focusedPlayer = nil
        npc:setFocus(nil)
    end
end)

-- Register the NPC type
npcType:register()

print("[NpcType] Registered test NPC: " .. npcName)
