-- QuestBuilder: Specialized NPC builder for multi-step quest NPCs
-- Extends NpcBuilder with storage-driven dialogue states, transitions,
-- and action execution (giveItem, removeItem, setStorage, grantOutfit, teleport).

QuestBuilder = setmetatable({}, { __index = NpcBuilder })
QuestBuilder.__index = QuestBuilder

function QuestBuilder:new(name, outfit)
    local obj = NpcBuilder.new(self, name, outfit)
    obj._greetStates = {}  -- { {storage, greetText, transitions}, ... }
    setmetatable(obj, self)
    return obj
end

function QuestBuilder:addState(config)
    -- config = {
    --   storage = {key, value},  -- player storage condition
    --   greetText = "...",       -- what NPC says on greet in this state
    --   transitions = {
    --     { keywords = {"yes"}, text = "...", actions = {...}, nextStorage = {key, val} },
    --     { keywords = {"no"}, text = "..." },
    --   }
    -- }
    self._greetStates[#self._greetStates + 1] = config
    return self
end

function QuestBuilder:addAction(transition, actionType, params)
    -- actionType: "giveItem", "removeItem", "setStorage", "grantOutfit", "teleport"
    if not transition.actions then transition.actions = {} end
    transition.actions[#transition.actions + 1] = { type = actionType, params = params }
    return self
end

-- Execute a list of actions on a player
function QuestBuilder:executeActions(player, actions)
    if not actions then return true end
    for _, action in ipairs(actions) do
        if action.type == "giveItem" then
            player:addItem(action.params.itemId, action.params.count or 1)
        elseif action.type == "removeItem" then
            if not player:removeItem(action.params.itemId, action.params.count or 1) then
                return false
            end
        elseif action.type == "setStorage" then
            player:setStorageValue(action.params.key, action.params.value)
        elseif action.type == "grantOutfit" then
            if action.params.addon then
                player:addOutfitAddon(action.params.lookType, action.params.addon)
            else
                player:addOutfit(action.params.lookType)
            end
        elseif action.type == "teleport" then
            player:teleportTo(Position(action.params.position))
            Position(action.params.position):sendMagicEffect(CONST_ME_TELEPORT)
        end
    end
    return true
end

-- Check prerequisites for a transition
function QuestBuilder:checkPrerequisites(player, transition)
    if not transition.prerequisites then return true end
    for _, prereq in ipairs(transition.prerequisites) do
        if prereq.type == "level" then
            if player:getLevel() < prereq.value then return false end
        elseif prereq.type == "storage" then
            if player:getStorageValue(prereq.key) ~= prereq.value then return false end
        elseif prereq.type == "item" then
            if player:getItemCount(prereq.itemId) < (prereq.count or 1) then return false end
        end
    end
    return true
end

-- Find the current quest state for a player based on storage values
function QuestBuilder:findCurrentState(player)
    -- Iterate in reverse so later (more advanced) states take priority
    for i = #self._greetStates, 1, -1 do
        local state = self._greetStates[i]
        if state.storage then
            local val = player:getStorageValue(state.storage[1])
            if val == state.storage[2] then
                return state
            end
        end
    end
    -- Check for a default state (no storage condition)
    for _, state in ipairs(self._greetStates) do
        if not state.storage then
            return state
        end
    end
    return nil
end

function QuestBuilder:greet(npc, player)
    local cid = player:getId()
    local npcId = npc:getId()

    -- Custom greet callback
    if self._onGreetCallback then
        local result = self._onGreetCallback(npc, player, self)
        if result == false then return end
    end

    self:addFocus(npc, cid)

    -- Find the current quest state for this player
    local currentState = self:findCurrentState(player)
    if currentState and currentState.greetText then
        local s = InstanceState.get(npcId, cid)
            or InstanceState.create(npcId, cid)
        s.questState = currentState

        -- Register transition keywords for this state
        local text = currentState.greetText:gsub("|PLAYERNAME|", player:getName())
        self:say(npc, text, player)
    else
        -- Fall back to default greet message
        local text = self._greetMsg:gsub("|PLAYERNAME|", player:getName())
        self:say(npc, text, player)
    end
end

function QuestBuilder:register()
    local builder = self

    -- Register a custom onSay handler for state-aware transitions
    local originalOnSay = self._onSayCallback
    self._onSayCallback = function(npc, player, message, b)
        local npcId = npc:getId()
        local cid = player:getId()
        local s = InstanceState.get(npcId, cid)
        if not s or not s.questState or not s.questState.transitions then
            if originalOnSay then
                return originalOnSay(npc, player, message, b)
            end
            return false
        end

        local lower = message:lower()
        for _, transition in ipairs(s.questState.transitions) do
            local matched = false
            for _, kw in ipairs(transition.keywords) do
                if lower:find(kw:lower(), 1, true) then
                    matched = true
                    break
                end
            end
            if matched then
                -- Check prerequisites
                if not builder:checkPrerequisites(player, transition) then
                    if transition.failText then
                        builder:say(npc, transition.failText:gsub("|PLAYERNAME|", player:getName()), player)
                    end
                    InstanceState.updateTalkStart(npcId, cid)
                    return true
                end

                -- Execute actions
                if transition.actions then
                    if not builder:executeActions(player, transition.actions) then
                        if transition.failText then
                            builder:say(npc, transition.failText:gsub("|PLAYERNAME|", player:getName()), player)
                        end
                        InstanceState.updateTalkStart(npcId, cid)
                        return true
                    end
                end

                -- Set next storage if defined
                if transition.nextStorage then
                    player:setStorageValue(transition.nextStorage[1], transition.nextStorage[2])
                end

                -- Say transition text
                if transition.text then
                    builder:say(npc, transition.text:gsub("|PLAYERNAME|", player:getName()), player)
                end

                -- Re-evaluate state after transition
                s.questState = builder:findCurrentState(player)
                InstanceState.updateTalkStart(npcId, cid)
                return true
            end
        end

        if originalOnSay then
            return originalOnSay(npc, player, message, b)
        end
        return false
    end

    NpcBuilder.register(self)
    return self
end
