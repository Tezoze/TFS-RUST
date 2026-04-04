-- InstanceState: Per-NPC-spawn, per-player interaction state management
-- All state keyed by spawned NPC creature ID (npc:getId()), NOT NPC name.
-- This ensures multi-spawn isolation (e.g., "Ongulf" in Svargrond vs Kazordoon).

InstanceState = {}

-- Internal storage: state[npcId][cid] = { topic = 0, ... }
local state = {}

-- Per-spawn focus lists: focuses[npcId] = { [cid] = true, ... }
local focuses = {}

-- Per-spawn talk timestamps: talkStart[npcId] = { [cid] = os.time(), ... }
local talkStart = {}

-- Per-spawn voice cooldown: lastVoice[npcId] = timestamp
local lastVoice = {}

-- Per-spawn pending multi-message events: pendingEvents[npcId][cid] = { eventId1, eventId2, ... }
local pendingEvents = {}

-- Registry of all spawned NPC IDs using the builder framework (for logout cleanup)
local registeredNpcs = {}

function InstanceState.register(npcId)
    -- Mark this spawned NPC ID as using the builder framework
    registeredNpcs[npcId] = true
    state[npcId] = state[npcId] or {}
    focuses[npcId] = focuses[npcId] or {}
    talkStart[npcId] = talkStart[npcId] or {}
end

function InstanceState.get(npcId, cid)
    -- Returns the state table for this NPC-player pair, or nil
    if state[npcId] then
        return state[npcId][cid]
    end
    return nil
end

function InstanceState.create(npcId, cid)
    -- Creates a fresh state entry for this NPC-player pair
    state[npcId] = state[npcId] or {}
    state[npcId][cid] = { topic = 0 }
    return state[npcId][cid]
end

function InstanceState.remove(npcId, cid)
    -- Removes the state entry for this NPC-player pair
    if state[npcId] then
        state[npcId][cid] = nil
    end
    if focuses[npcId] then
        focuses[npcId][cid] = nil
    end
    if talkStart[npcId] then
        talkStart[npcId][cid] = nil
    end
    if pendingEvents[npcId] then
        pendingEvents[npcId][cid] = nil
    end
end

function InstanceState.removePlayer(cid)
    -- Removes state entries for this CID across all spawned NPCs
    -- Called on player logout/disconnect
    -- NOTE: We intentionally do NOT clear focuses here.
    -- The NPC's handleDisappear/handleThink will detect the player is gone
    -- and call releaseFocus, which properly resets the NPC's visual focus.
    for npcId, _ in pairs(registeredNpcs) do
        if state[npcId] then state[npcId][cid] = nil end
        if talkStart[npcId] then talkStart[npcId][cid] = nil end
        if pendingEvents[npcId] then pendingEvents[npcId][cid] = nil end
    end
end

-- Focus management (per-spawn)
function InstanceState.isFocused(npcId, cid)
    return focuses[npcId] and focuses[npcId][cid] == true
end

function InstanceState.setFocus(npcId, cid)
    focuses[npcId] = focuses[npcId] or {}
    focuses[npcId][cid] = true
    talkStart[npcId] = talkStart[npcId] or {}
    talkStart[npcId][cid] = os.time()
end

function InstanceState.clearFocus(npcId, cid)
    if focuses[npcId] then focuses[npcId][cid] = nil end
    if talkStart[npcId] then talkStart[npcId][cid] = nil end
end

function InstanceState.getFocuses(npcId)
    return focuses[npcId] or {}
end

function InstanceState.getTalkStart(npcId, cid)
    return talkStart[npcId] and talkStart[npcId][cid]
end

function InstanceState.updateTalkStart(npcId, cid)
    talkStart[npcId] = talkStart[npcId] or {}
    talkStart[npcId][cid] = os.time()
end

-- Voice cooldown (per-spawn)
function InstanceState.getLastVoice(npcId)
    return lastVoice[npcId] or 0
end

function InstanceState.setLastVoice(npcId, time)
    lastVoice[npcId] = time
end

-- Pending events management (for multi-message cancellation)
function InstanceState.getPendingEvents(npcId, cid)
    if pendingEvents[npcId] then
        return pendingEvents[npcId][cid]
    end
    return nil
end

function InstanceState.setPendingEvents(npcId, cid, events)
    pendingEvents[npcId] = pendingEvents[npcId] or {}
    pendingEvents[npcId][cid] = events
end

function InstanceState.clearPendingEvents(npcId, cid)
    if pendingEvents[npcId] then
        pendingEvents[npcId][cid] = nil
    end
end

function InstanceState.getRegisteredNpcs()
    return registeredNpcs
end
