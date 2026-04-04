-- Table to track recent kills per player to prevent double counting
local recentKills = {}

-- Configuration
local DAMAGE_THRESHOLD = 0.10 -- 10% damage required for shared credit
local PARTY_SHARE_RANGE = 30 -- Distance in sqm for party credit
local SOLO_KILL_THRESHOLD = 0.51 -- 51% damage = exclusive credit

-- Optimize task lookup: create creature-to-task mapping
local creatureToTask = {}
if tasks then
    for taskId, task in pairs(tasks) do
        for _, creature in ipairs(task.creatures) do
            local lowerName = creature:lower()
            creatureToTask[lowerName] = creatureToTask[lowerName] or {}
            table.insert(creatureToTask[lowerName], taskId)
        end
    end
end

local function giveTaskCredit(player, targetName, targetId)
    local playerGuid = player:getGuid()
    
    -- Deduplication check
    if not recentKills[playerGuid] then
        recentKills[playerGuid] = {}
    end
    
    if recentKills[playerGuid][targetId] then
        return
    end
    
    recentKills[playerGuid][targetId] = true
    
    addEvent(function(guid, tid)
        if recentKills[guid] then
            recentKills[guid][tid] = nil
            if not next(recentKills[guid]) then
                recentKills[guid] = nil
            end
        end
    end, 1000, playerGuid, targetId)
    
    -- Process task credit
    local taskIds = creatureToTask[targetName] or {}
    for _, taskId in ipairs(taskIds) do
        if isInArray(player:getStartedTasks(), taskId) then
            local storageKey = Storage.KillingInTheNameOf.KillsStorageBase + taskId
            local killAmount = math.max(0, player:getStorageValue(storageKey))
            
            if killAmount < tasks[taskId].killsRequired then
                player:setStorageValue(storageKey, killAmount + 1)
                player:sendTextMessage(MESSAGE_STATUS_CONSOLE_BLUE, 'Task progress: ' .. (killAmount + 1) .. "/" .. tasks[taskId].killsRequired .. " " .. tasks[taskId].raceName)
            end
        end
    end
end

function onKill(creature, target)
    local player = creature:getPlayer()
    if not player then
        return true
    end

    if target:isPlayer() or target:getMaster() then
        return true
    end

    local targetId = target:getId()
    local targetName = target:getName():lower()

    -- 1. Raymond Striker's pirates task
    if player:getStorageValue(Storage.KillingInTheNameOf.RaymondPirates) == 1 then
        local pirateCreatures = {"pirate ghost", "pirate marauder", "pirate cutthroad", "pirate buccaneer", "pirate corsair", "pirate skeleton"}
        if isInArray(pirateCreatures, targetName) then
            local currentKills = math.max(0, player:getStorageValue(Storage.KillingInTheNameOf.RaymondPiratesCount))
            if currentKills < 3000 then
                player:setStorageValue(Storage.KillingInTheNameOf.RaymondPiratesCount, currentKills + 1)
                player:sendTextMessage(MESSAGE_STATUS_CONSOLE_BLUE, 'Pirates task progress: ' .. (currentKills + 1) .. "/3000 pirates")
            end
        end
    end

    -- 2. Special Boss Completion (Demodras)
    if targetName == "demodras" and player:getStorageValue(Storage.KillingInTheNameOf.MissionDemodras) == 1 then
        player:setStorageValue(Storage.KillingInTheNameOf.MissionDemodras, 2) -- 2 = completed
        player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "You have defeated Demodras!")
    end

    -- 3. Special Boss Completion (Tiquandas Revenge)
    if targetName == "tiquandas revenge" and player:getStorageValue(Storage.KillingInTheNameOf.MissionTiquandasRevenge) == 1 then
        player:setStorageValue(Storage.KillingInTheNameOf.MissionTiquandasRevenge, 2) -- 2 = completed
        player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "You have defeated Tiquandas Revenge!")
    end

    -- 4. Damage Calculation
    local targetMaxHealth = target:getMaxHealth()
    local damageMap = target:getDamageMap()
    
    local eligiblePlayers = {}
    local killerDamage = 0
    
    -- Calculate killer's damage percentage
    if damageMap[player:getGuid()] then
        killerDamage = damageMap[player:getGuid()].total / targetMaxHealth
    end
    
    -- Scenario 1: Killer dealt majority damage (51%+) = SOLO CREDIT ONLY
    if killerDamage >= SOLO_KILL_THRESHOLD then
        eligiblePlayers[player:getGuid()] = player
    
    -- Scenario 2: Killer is in party = PARTY SHARE
    elseif player:getParty() then
        local party = player:getParty()
        local members = party:getMembers()
        members[#members + 1] = party:getLeader()
        
        local targetPos = target:getPosition()
        for _, member in ipairs(members) do
            if member and member:getPosition():getDistance(targetPos) <= PARTY_SHARE_RANGE then
                eligiblePlayers[member:getGuid()] = member
            end
        end
    
    -- Scenario 3: Killer dealt minority damage = CREDIT ALL SIGNIFICANT CONTRIBUTORS
    else
        for attackerId, damage in pairs(damageMap) do
            local damagePercent = damage.total / targetMaxHealth
            if damagePercent >= DAMAGE_THRESHOLD then
                local attacker = Player(attackerId)
                if attacker then
                    eligiblePlayers[attacker:getGuid()] = attacker
                end
            end
        end
    end
    
    -- Give credit to all eligible players
    for _, eligiblePlayer in pairs(eligiblePlayers) do
        giveTaskCredit(eligiblePlayer, targetName, targetId)
    end
    
    return true
end

function onLogout(player)
    local playerGuid = player:getGuid()
    recentKills[playerGuid] = nil
    return true
end
