--[[
    Server-Side Autoloot System
    Uses extended opcode 202 for client<->server communication.
    Loot rules are persisted in player storage values.
    
    Storage layout (per player):
      STORAGE_BASE (60000)       = enabled (0/1)
      STORAGE_BASE + 1           = lootAll (0/1)
      STORAGE_BASE + 2           = rule count
      STORAGE_BASE + 3 + (i*2)   = rule i itemId   (server ID)
      STORAGE_BASE + 4 + (i*2)   = rule i containerId (server ID)
    
    Max 20 rules per player (uses storage keys 60000–60044).
]]

local AUTOLOOT_OPCODE = 202
local STORAGE_BASE = 60000
local MAX_RULES = 20

local CONFIG = {
    DEBUG = false, -- Set to true to enable debug logging
}

local function debugLog(message)
    if CONFIG.DEBUG then
        print("[AutoLoot DEBUG] " .. message)
    end
end

-- ============================================================
-- Storage Helpers
-- ============================================================

local function getAutoLootEnabled(player)
    return player:getStorageValue(STORAGE_BASE) == 1
end

local function setAutoLootEnabled(player, enabled)
    player:setStorageValue(STORAGE_BASE, enabled and 1 or 0)
end

local function getLootAll(player)
    return player:getStorageValue(STORAGE_BASE + 1) == 1
end

local function setLootAll(player, lootAll)
    player:setStorageValue(STORAGE_BASE + 1, lootAll and 1 or 0)
end

local function getRuleCount(player)
    return math.max(0, player:getStorageValue(STORAGE_BASE + 2))
end

local function setRuleCount(player, count)
    player:setStorageValue(STORAGE_BASE + 2, count)
end

local function getRule(player, index)
    local itemId = player:getStorageValue(STORAGE_BASE + 3 + (index * 2))
    local containerId = player:getStorageValue(STORAGE_BASE + 4 + (index * 2))
    if itemId > 0 and containerId > 0 then
        return {itemId = itemId, containerId = containerId}
    end
    return nil
end

local function setRule(player, index, itemId, containerId)
    player:setStorageValue(STORAGE_BASE + 3 + (index * 2), itemId)
    player:setStorageValue(STORAGE_BASE + 4 + (index * 2), containerId)
end

local function clearRule(player, index)
    player:setStorageValue(STORAGE_BASE + 3 + (index * 2), -1)
    player:setStorageValue(STORAGE_BASE + 4 + (index * 2), -1)
end

--- Get all rules as a table
local function getAllRules(player)
    local rules = {}
    local count = getRuleCount(player)
    for i = 0, count - 1 do
        local rule = getRule(player, i)
        if rule then
            table.insert(rules, rule)
        end
    end
    return rules
end

--- Save a full rule set (replaces all existing rules)
local function saveAllRules(player, rules)
    -- Clear old rules
    local oldCount = getRuleCount(player)
    for i = 0, oldCount - 1 do
        clearRule(player, i)
    end
    
    -- Write new rules (capped at MAX_RULES)
    local count = math.min(#rules, MAX_RULES)
    for i = 0, count - 1 do
        setRule(player, i, rules[i + 1].itemId, rules[i + 1].containerId)
    end
    setRuleCount(player, count)
end

-- ============================================================
-- Autoloot Core Logic
-- Called from Monster:onDropLoot after loot is placed in corpse
-- ============================================================

function doAutoLoot(player, corpse)
    debugLog("doAutoLoot called for player: " .. (player and player:getName() or "nil") .. ", corpse: " .. (corpse and tostring(corpse:getId()) or "nil"))
    if not player or not corpse then
        debugLog("player or corpse is nil, aborting")
        return
    end
    
    if not getAutoLootEnabled(player) then
        debugLog("Autoloot is DISABLED for " .. player:getName())
        return
    end
    debugLog("Autoloot is ENABLED for " .. player:getName())
    
    local lootAll = getLootAll(player)
    local rules = getAllRules(player)
    
    debugLog("lootAll: " .. tostring(lootAll) .. ", rule count: " .. #rules)
    
    -- Build a fast lookup: itemId -> containerId
    local itemToContainer = {}
    local containerIds = {}
    for _, rule in ipairs(rules) do
        itemToContainer[rule.itemId] = rule.containerId
        containerIds[rule.containerId] = true
        debugLog("Rule: itemId " .. rule.itemId .. " -> containerId " .. rule.containerId)
    end
    
    -- If no rules and lootAll is off, nothing to do
    if not lootAll and next(itemToContainer) == nil then
        debugLog("No rules and lootAll is off, nothing to do")
        return
    end
    
    -- Find target containers in the player's inventory
    -- We search the player's backpack slot (slot 3) recursively
    local function findContainerByItemId(targetItemId)
        local backpack = player:getSlotItem(CONST_SLOT_BACKPACK)
        if not backpack then return nil end
        
        local container = backpack:getContainer()
        if not container then return nil end
        
        -- Check if backpack itself matches
        if backpack:getId() == targetItemId then
            if container:getEmptySlots(false) > 0 then
                return container
            end
        end
        
        -- Search inside the backpack for nested containers
        local queue = {container}
        while #queue > 0 do
            local current = table.remove(queue, 1)
            for i = 0, current:getSize() - 1 do
                local item = current:getItem(i)
                if item then
                    local subContainer = item:getContainer()
                    if subContainer then
                        if item:getId() == targetItemId then
                            if subContainer:getEmptySlots(false) > 0 then
                                return subContainer
                            end
                        end
                        table.insert(queue, subContainer)
                    end
                end
            end
        end
        
        return nil
    end
    
    -- Get the default (main backpack) container as fallback
    local function getDefaultContainer()
        local backpack = player:getSlotItem(CONST_SLOT_BACKPACK)
        if not backpack then return nil end
        local container = backpack:getContainer()
        if container and container:getEmptySlots(false) > 0 then
            return container
        end
        -- If main backpack is full, try nested containers
        if container then
            local queue = {container}
            while #queue > 0 do
                local current = table.remove(queue, 1)
                for i = 0, current:getSize() - 1 do
                    local item = current:getItem(i)
                    if item then
                        local sub = item:getContainer()
                        if sub then
                            if sub:getEmptySlots(false) > 0 then
                                return sub
                            end
                            table.insert(queue, sub)
                        end
                    end
                end
            end
        end
        return nil
    end
    
    -- Cache containers we've already found
    local containerCache = {}
    
    local function getTargetContainer(itemId)
        local targetContainerId = itemToContainer[itemId]
        
        if targetContainerId then
            -- Try to find the specific container type
            if containerCache[targetContainerId] == nil then
                containerCache[targetContainerId] = findContainerByItemId(targetContainerId) or false
            end
            if containerCache[targetContainerId] then
                -- Verify it still has space
                if containerCache[targetContainerId]:getEmptySlots(false) > 0 then
                    return containerCache[targetContainerId]
                end
                -- Refresh cache if full
                containerCache[targetContainerId] = findContainerByItemId(targetContainerId) or false
                if containerCache[targetContainerId] then
                    return containerCache[targetContainerId]
                end
            end
        end
        
        -- Fallback to default container
        return getDefaultContainer()
    end
    
    -- Collect items to autoloot (iterate forward, store info, process after)
    local itemsToLoot = {}
    local corpseSize = corpse:getSize()
    
    debugLog("Corpse has " .. corpseSize .. " items")
    
    for i = 0, corpseSize - 1 do
        local item = corpse:getItem(i)
        if item then
            local itemId = item:getId()
            local shouldLoot = lootAll or (itemToContainer[itemId] ~= nil)
            local itemType = ItemType(itemId)
            debugLog("Corpse item [" .. i .. "]: " .. (itemType and itemType:getName() or "?") .. " (id:" .. itemId .. ") x" .. item:getCount() .. " -> " .. (shouldLoot and "LOOT" or "skip"))
            
            if shouldLoot then
                table.insert(itemsToLoot, {
                    item = item,
                    itemId = itemId,
                    count = item:getCount(),
                    actionId = item:getActionId()
                })
            end
        end
    end
    
    debugLog("Items to loot: " .. #itemsToLoot)
    
    -- Process in reverse order so removal doesn't shift indices
    local lootedItems = {}
    for i = #itemsToLoot, 1, -1 do
        local info = itemsToLoot[i]
        local item = info.item
        local itemId = info.itemId
        local count = info.count
        
        local targetContainer = getTargetContainer(itemId)
        if targetContainer then
            -- Check player capacity
            local itemType = ItemType(itemId)
            local itemWeight = itemType:getWeight(count)
            local freeCap = player:getFreeCapacity()
            
            debugLog("Trying to move " .. (itemType and itemType:getName() or "?") .. " (id:" .. itemId .. ") x" .. count .. " | weight: " .. itemWeight .. " | freeCap: " .. freeCap)
            
            if freeCap >= itemWeight then
                -- Try to add the item directly to the target container
                local ret = targetContainer:addItemEx(item)
                debugLog("addItemEx result: " .. tostring(ret) .. " (NOERROR=" .. tostring(RETURNVALUE_NOERROR) .. ")")
                if ret == RETURNVALUE_NOERROR then
                    local itemName = itemType:getName()
                    if count > 1 then
                        table.insert(lootedItems, 1, count .. " " .. itemType:getPluralName())
                    else
                        table.insert(lootedItems, 1, itemName)
                    end
                else
                    debugLog("FAILED to move item, return value: " .. tostring(ret))
                end
            else
                debugLog("SKIPPED - not enough capacity (need " .. itemWeight .. ", have " .. freeCap .. ")")
            end
        else
            debugLog("No target container found for itemId " .. itemId)
        end
    end
    
    -- Send autoloot summary message
    if #lootedItems > 0 then
        local summary = "Autolooted: " .. table.concat(lootedItems, ", ") .. "."
        player:sendTextMessage(MESSAGE_LOOT, summary)
    end
end

-- ============================================================
-- Extended Opcode Handler — called from extendedopcode.lua
-- This is a global function so it can be found by the main
-- extended opcode dispatcher in creaturescripts/scripts/extendedopcode.lua
-- ============================================================

function handleAutoLootOpcode(player, opcode, buffer)
    debugLog("Received opcode 202 from " .. player:getName() .. ", buffer length: " .. #buffer)
    debugLog("Buffer: " .. buffer)
    
    local status, data = pcall(json.decode, buffer)
    if not status or not data then
        debugLog("Failed to decode JSON from " .. player:getName())
        return
    end
    
    debugLog("Action: " .. tostring(data.action))
    
    if data.action == "sync" then
        -- Client sends its full rule set
        local enabled = data.enabled or false
        local lootAll = data.lootAll or false
        local rules = data.rules or {}
        
        setAutoLootEnabled(player, enabled)
        setLootAll(player, lootAll)
        
        -- Validate and save rules
        local validRules = {}
        for _, rule in ipairs(rules) do
            if type(rule.itemId) == "number" and type(rule.containerId) == "number" then
                table.insert(validRules, {
                    itemId = rule.itemId,
                    containerId = rule.containerId
                })
            end
        end
        
        saveAllRules(player, validRules)
        debugLog("Saved " .. #validRules .. " rules for " .. player:getName() .. " | enabled: " .. tostring(enabled) .. " | lootAll: " .. tostring(lootAll))
        for i, r in ipairs(validRules) do
            debugLog("  Rule " .. i .. ": itemId=" .. r.itemId .. " containerId=" .. r.containerId)
        end
        player:sendTextMessage(MESSAGE_STATUS_CONSOLE_BLUE, 
            "Autoloot " .. (enabled and "enabled" or "disabled") .. 
            " with " .. #validRules .. " rules" ..
            (lootAll and " (Loot All mode)" or "") .. ".")
        
    elseif data.action == "refresh" then
        -- Server sends current rules back to client
        local rules = getAllRules(player)
        local payload = {
            action = "sync",
            enabled = getAutoLootEnabled(player),
            lootAll = getLootAll(player),
            rules = rules
        }
        player:sendExtendedOpcode(AUTOLOOT_OPCODE, json.encode(payload))
    end
end

-- ============================================================
-- Login Event — send stored autoloot state to client
-- ============================================================

local loginEvent = CreatureEvent("AutoLootLogin")

function loginEvent.onLogin(player)
    debugLog("Login event for " .. player:getName())
    
    -- Send current autoloot state to client on login
    -- Small delay to ensure client module is loaded
    addEvent(function(playerId)
        local p = Player(playerId)
        if p then
            local rules = getAllRules(p)
            local payload = {
                action = "sync",
                enabled = getAutoLootEnabled(p),
                lootAll = getLootAll(p),
                rules = rules
            }
            debugLog("Sending " .. #rules .. " rules to " .. p:getName() .. " on login")
            p:sendExtendedOpcode(AUTOLOOT_OPCODE, json.encode(payload))
        end
    end, 2000, player:getId())
    
    return true
end

loginEvent:register()

print("[AutoLoot] Server-side autoloot system loaded (opcode " .. AUTOLOOT_OPCODE .. ")")
