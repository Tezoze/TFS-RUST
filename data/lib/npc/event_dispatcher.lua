-- EventDispatcher: Centralized NpcType callback registration
-- Registers the five standard NpcType callbacks for each builder-created NPC.
-- Routes events to the correct NpcBuilder instance.
-- Each callback calls setCurrentNpc(npc) before dispatching.

EventDispatcher = {}

-- Registry: npcName -> builder instance
local builders = {}

function EventDispatcher.register(npcName, builder)
    builders[npcName] = builder
end

function EventDispatcher.get(npcName)
    return builders[npcName]
end

function EventDispatcher.setupCallbacks(npcType, npcName)
    -- onAppear
    npcType:eventType(NPCS_EVENT_APPEAR)
    npcType:onAppear(function(npc, creature)
        setCurrentNpc(npc)
        local b = builders[npcName]
        if b then b:handleAppear(npc, creature) end
    end)

    -- onDisappear
    npcType:eventType(NPCS_EVENT_DISAPPEAR)
    npcType:onDisappear(function(npc, creature)
        setCurrentNpc(npc)
        local b = builders[npcName]
        if b then b:handleDisappear(npc, creature) end
    end)

    -- onSay
    npcType:eventType(NPCS_EVENT_SAY)
    npcType:onSay(function(npc, creature, msgtype, message)
        setCurrentNpc(npc)
        local b = builders[npcName]
        if b then b:handleSay(npc, creature, msgtype, message) end
    end)

    -- onThink
    npcType:eventType(NPCS_EVENT_THINK)
    npcType:onThink(function(npc, interval)
        setCurrentNpc(npc)
        local b = builders[npcName]
        if b then b:handleThink(npc, interval) end
    end)

    -- onCloseChannel
    npcType:eventType(NPCS_EVENT_CLOSECHANNEL)
    npcType:onCloseChannel(function(npc, creature)
        setCurrentNpc(npc)
        local b = builders[npcName]
        if b then b:handleCloseChannel(npc, creature) end
    end)
end
