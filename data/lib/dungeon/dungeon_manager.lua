--[[
    Dungeon System — Instance Manager
    Handles instance lifecycle: create, assign, cleanup, teleport, reconnect.
]]

DungeonManager = {
    activeInstances = {},  -- [slotKey] = instance data
    playerToSlot = {},     -- [playerId] = slotKey
    MAX_TIME = 3600,
    CLEANUP_INTERVAL = 60,
    DISCONNECT_GRACE = 300, -- 5 minutes
}

-- Slot key = "dungeonId_slotIndex" for uniqueness
function DungeonManager.slotKey(dungeonId, slotIndex)
    return dungeonId .. "_" .. slotIndex
end

function DungeonManager.getAbsolutePos(slotId, dungeonId, relativePos)
    local slot = DungeonConfig[dungeonId].slots[slotId]
    if not slot then return nil end
    return Position(
        slot.entrance.x + (relativePos.dx or 0),
        slot.entrance.y + (relativePos.dy or 0),
        slot.entrance.z + (relativePos.dz or 0)
    )
end

function DungeonManager.findFreeSlot(dungeonId)
    local dungeon = DungeonConfig[dungeonId]
    if not dungeon then return nil end
    for slotIndex, _ in pairs(dungeon.slots) do
        local key = DungeonManager.slotKey(dungeonId, slotIndex)
        if not DungeonManager.activeInstances[key] then
            return slotIndex
        end
    end
    return nil
end

function DungeonManager.isInDungeon(player)
    return DungeonManager.playerToSlot[player:getId()] ~= nil
end

function DungeonManager.getInstanceByPlayer(player)
    local key = DungeonManager.playerToSlot[player:getId()]
    if not key then return nil end
    return DungeonManager.activeInstances[key], key
end

function DungeonManager.enterDungeon(leader, dungeonId)
    local dungeon = DungeonConfig[dungeonId]
    if not dungeon then
        leader:sendTextMessage(MESSAGE_STATUS_SMALL, "Unknown dungeon.")
        return false
    end

    -- Gather party members
    local party = leader:getParty()
    local players = {}
    if party then
        players = party:getMembers()
        table.insert(players, party:getLeader())
    else
        table.insert(players, leader)
    end

    -- Validate player count
    if #players < (dungeon.minPlayers or 1) then
        leader:sendTextMessage(MESSAGE_STATUS_SMALL, "You need at least " .. (dungeon.minPlayers or 1) .. " party members.")
        return false
    end
    if #players > dungeon.maxPlayers then
        leader:sendTextMessage(MESSAGE_STATUS_SMALL, "Maximum " .. dungeon.maxPlayers .. " players allowed.")
        return false
    end

    -- Validate all players
    for _, p in ipairs(players) do
        if p:getLevel() < dungeon.minLevel then
            leader:sendTextMessage(MESSAGE_STATUS_SMALL, p:getName() .. " does not meet the level requirement (" .. dungeon.minLevel .. ").")
            return false
        end
        if DungeonManager.isInDungeon(p) then
            leader:sendTextMessage(MESSAGE_STATUS_SMALL, p:getName() .. " is already in a dungeon.")
            return false
        end
    end

    -- Check lockout
    if DungeonManager.hasLockout(leader, dungeonId, "daily") then
        leader:sendTextMessage(MESSAGE_STATUS_SMALL, "You have already completed this dungeon today.")
        return false
    end

    -- Find free slot
    local slotIndex = DungeonManager.findFreeSlot(dungeonId)
    if not slotIndex then
        leader:sendTextMessage(MESSAGE_STATUS_SMALL, "All dungeon slots are in use. Try again later.")
        return false
    end

    local key = DungeonManager.slotKey(dungeonId, slotIndex)
    local slot = dungeon.slots[slotIndex]

    -- Create instance
    local instance = {
        dungeonId = dungeonId,
        slotIndex = slotIndex,
        slotKey = key,
        partyLeaderId = leader:getId(),
        players = {},
        startTime = os.time(),
        encounterIndex = 0,
        currentPhase = 0,
        spawnedCreatures = {},
        deaths = 0,
        completed = false,
        encounterActive = false,
    }

    for _, p in ipairs(players) do
        table.insert(instance.players, {
            id = p:getId(),
            name = p:getName(),
            connected = true,
        })
    end

    DungeonManager.activeInstances[key] = instance

    -- Teleport all players in
    for _, p in ipairs(players) do
        -- Save return position
        p:setStorageValue(Storage.Dungeon.SavedPosX, p:getPosition().x)
        p:setStorageValue(Storage.Dungeon.SavedPosY, p:getPosition().y)
        p:setStorageValue(Storage.Dungeon.SavedPosZ, p:getPosition().z)
        p:setStorageValue(Storage.Dungeon.CurrentInstance, slotIndex)
        p:setStorageValue(Storage.Dungeon.CurrentDungeonId, dungeonId)

        DungeonManager.playerToSlot[p:getId()] = key
        p:teleportTo(slot.entrance)
        slot.entrance:sendMagicEffect(CONST_ME_TELEPORT)
        p:sendTextMessage(MESSAGE_EVENT_ADVANCE, "You have entered " .. dungeon.name .. "!")
    end

    -- Start timer
    local timeLimit = dungeon.timeLimit or DungeonManager.MAX_TIME
    addEvent(DungeonManager.onTimeout, timeLimit * 1000, key)

    -- Trigger first encounter if proximity-based
    addEvent(DungeonManager.checkProximityTriggers, 2000, key)

    -- Sync UI
    DungeonManager.broadcastInstanceStart(key)

    return true
end

function DungeonManager.leaveDungeon(player)
    local instance, key = DungeonManager.getInstanceByPlayer(player)
    if not instance then return false end

    DungeonManager.removePlayerFromInstance(player, key)
    DungeonManager.teleportPlayerOut(player)

    -- If no players left, cleanup
    local hasPlayers = false
    for _, pd in ipairs(instance.players) do
        if pd.connected then hasPlayers = true break end
    end
    if not hasPlayers then
        DungeonManager.cleanupSlot(key)
    end

    return true
end

function DungeonManager.removePlayerFromInstance(player, key)
    local instance = DungeonManager.activeInstances[key]
    if not instance then return end

    for _, pd in ipairs(instance.players) do
        if pd.id == player:getId() then
            pd.connected = false
            break
        end
    end

    DungeonManager.playerToSlot[player:getId()] = nil
    player:setStorageValue(Storage.Dungeon.CurrentInstance, 0)
    player:setStorageValue(Storage.Dungeon.CurrentDungeonId, 0)
end

function DungeonManager.teleportPlayerOut(player)
    local savedX = player:getStorageValue(Storage.Dungeon.SavedPosX)
    local savedY = player:getStorageValue(Storage.Dungeon.SavedPosY)
    local savedZ = player:getStorageValue(Storage.Dungeon.SavedPosZ)

    if savedX > 0 and savedY > 0 then
        player:teleportTo(Position(savedX, savedY, savedZ))
    else
        player:teleportTo(player:getTown():getTemplePosition())
    end
    player:getPosition():sendMagicEffect(CONST_ME_TELEPORT)
end

function DungeonManager.cleanupSlot(key)
    local instance = DungeonManager.activeInstances[key]
    if not instance then return end

    local dungeon = DungeonConfig[instance.dungeonId]
    local slot = dungeon.slots[instance.slotIndex]

    -- Remove all spawned creatures
    for _, cid in ipairs(instance.spawnedCreatures) do
        local creature = Creature(cid)
        if creature then creature:remove() end
    end

    -- Remove any remaining players
    for _, pd in ipairs(instance.players) do
        local player = Player(pd.id)
        if player and DungeonManager.playerToSlot[pd.id] == key then
            DungeonManager.removePlayerFromInstance(player, key)
            DungeonManager.teleportPlayerOut(player)
        end
    end

    -- Clean items in the area
    if slot.fromPos and slot.toPos then
        for x = slot.fromPos.x, slot.toPos.x do
            for y = slot.fromPos.y, slot.toPos.y do
                local tile = Tile(Position(x, y, slot.fromPos.z))
                if tile then
                    local items = tile:getItems()
                    if items then
                        for _, item in ipairs(items) do
                            if item:getActionId() == 0 then
                                item:remove()
                            end
                        end
                    end
                end
            end
        end
    end

    DungeonManager.activeInstances[key] = nil
    print("[Dungeon] Cleaned up slot: " .. key)
end

function DungeonManager.onTimeout(key)
    local instance = DungeonManager.activeInstances[key]
    if not instance then return end
    if instance.completed then return end

    -- Notify and kick
    DungeonManager.broadcastToInstance(key, {
        action = "dungeon_failed",
        reason = "Time expired!",
    })

    addEvent(DungeonManager.cleanupSlot, 5000, key)
end

function DungeonManager.completeDungeon(key)
    local instance = DungeonManager.activeInstances[key]
    if not instance or instance.completed then return end
    instance.completed = true

    local dungeon = DungeonConfig[instance.dungeonId]
    local elapsed = os.time() - instance.startTime
    local score = DungeonManager.calculateScore(instance.deaths, elapsed, dungeon.timeLimit)

    -- Award rewards to all connected players
    for _, pd in ipairs(instance.players) do
        local player = Player(pd.id)
        if player and pd.connected then
            if dungeon.completionRewards.experience then
                player:addExperience(dungeon.completionRewards.experience, false)
            end
            if dungeon.completionRewards.money then
                player:addMoney(dungeon.completionRewards.money)
            end
            -- Set lockout
            player:setStorageValue(Storage.Dungeon.DailyLockoutBase + instance.dungeonId, os.time())
            -- Track stats
            local total = player:getStorageValue(Storage.Dungeon.TotalDungeonsRun)
            player:setStorageValue(Storage.Dungeon.TotalDungeonsRun, math.max(0, total) + 1)
        end
    end

    -- Broadcast completion
    DungeonManager.broadcastToInstance(key, {
        action = "dungeon_complete",
        dungeonName = dungeon.name,
        time = elapsed,
        deaths = instance.deaths,
        score = score,
        rewards = dungeon.completionRewards,
    })

    -- Cleanup after delay
    addEvent(DungeonManager.cleanupSlot, 30000, key)
end

function DungeonManager.calculateScore(deaths, elapsed, timeLimit)
    local timeRatio = elapsed / timeLimit
    if deaths == 0 and timeRatio < 0.5 then return "S" end
    if deaths <= 1 and timeRatio < 0.7 then return "A" end
    if deaths <= 3 and timeRatio < 0.85 then return "B" end
    if deaths <= 5 then return "C" end
    return "D"
end

function DungeonManager.hasLockout(player, dungeonId, lockoutType)
    local storageKey
    if lockoutType == "daily" then
        storageKey = Storage.Dungeon.DailyLockoutBase + dungeonId
    else
        storageKey = Storage.Dungeon.WeeklyLockoutBase + dungeonId
    end
    local lastRun = player:getStorageValue(storageKey)
    if lastRun <= 0 then return false end
    if lockoutType == "daily" then
        return os.date("%j", lastRun) == os.date("%j", os.time())
    else
        return os.date("%W", lastRun) == os.date("%W", os.time())
    end
end

function DungeonManager.checkProximityTriggers(key)
    local instance = DungeonManager.activeInstances[key]
    if not instance or instance.completed then return end
    if instance.encounterActive then
        addEvent(DungeonManager.checkProximityTriggers, 3000, key)
        return
    end

    local nextIndex = instance.encounterIndex + 1
    local dungeon = DungeonConfig[instance.dungeonId]
    local encounter = dungeon.encounters[nextIndex]
    if not encounter then return end

    if encounter.triggerType == "proximity" then
        local triggerPos = DungeonManager.getAbsolutePos(instance.slotIndex, instance.dungeonId, encounter.triggerPos)
        if triggerPos then
            for _, pd in ipairs(instance.players) do
                local player = Player(pd.id)
                if player and pd.connected then
                    if player:getPosition():getDistance(triggerPos) <= (encounter.triggerRadius or 5) then
                        DungeonEncounter.startEncounter(key, nextIndex)
                        return
                    end
                end
            end
        end
    elseif encounter.triggerType == "kill_previous" then
        if instance.encounterIndex == 0 and nextIndex == 1 then
            -- First encounter with kill_previous, auto-start
            DungeonEncounter.startEncounter(key, nextIndex)
            return
        end
    end

    addEvent(DungeonManager.checkProximityTriggers, 3000, key)
end

function DungeonManager.onCreatureDeath(key, creatureId)
    local instance = DungeonManager.activeInstances[key]
    if not instance then return end

    -- Remove from tracked creatures
    for i, cid in ipairs(instance.spawnedCreatures) do
        if cid == creatureId then
            table.remove(instance.spawnedCreatures, i)
            break
        end
    end

    -- Check if current encounter is cleared
    DungeonEncounter.checkEncounterCleared(key)
end

function DungeonManager.broadcastToInstance(key, data)
    local instance = DungeonManager.activeInstances[key]
    if not instance then return end
    local encoded = json.encode(data)
    for _, pd in ipairs(instance.players) do
        local player = Player(pd.id)
        if player and pd.connected then
            player:sendExtendedOpcode(210, encoded)
        end
    end
end

function DungeonManager.broadcastInstanceStart(key)
    local instance = DungeonManager.activeInstances[key]
    if not instance then return end
    local dungeon = DungeonConfig[instance.dungeonId]

    local playerList = {}
    for _, pd in ipairs(instance.players) do
        local player = Player(pd.id)
        if player then
            table.insert(playerList, {
                name = pd.name,
                level = player:getLevel(),
                vocation = player:getVocation():getName(),
            })
        end
    end

    DungeonManager.broadcastToInstance(key, {
        action = "instance_start",
        dungeonId = instance.dungeonId,
        dungeonName = dungeon.name,
        timeLimit = dungeon.timeLimit,
        difficulty = dungeon.difficulty,
        players = playerList,
        totalEncounters = #dungeon.encounters,
    })
end

-- Reconnect handler
function DungeonManager.onPlayerLogin(player)
    local slotIndex = player:getStorageValue(Storage.Dungeon.CurrentInstance)
    local dungeonId = player:getStorageValue(Storage.Dungeon.CurrentDungeonId)
    if slotIndex <= 0 or dungeonId <= 0 then return end

    local key = DungeonManager.slotKey(dungeonId, slotIndex)
    local instance = DungeonManager.activeInstances[key]

    if instance then
        -- Reconnect
        for _, pd in ipairs(instance.players) do
            if pd.id == player:getId() then
                pd.connected = true
                break
            end
        end
        DungeonManager.playerToSlot[player:getId()] = key
        player:sendTextMessage(MESSAGE_INFO_DESCR, "Reconnected to dungeon instance.")

        -- Sync UI
        local dungeon = DungeonConfig[dungeonId]
        local elapsed = os.time() - instance.startTime
        local remaining = (dungeon.timeLimit or DungeonManager.MAX_TIME) - elapsed
        player:sendExtendedOpcode(210, json.encode({
            action = "instance_start",
            dungeonId = dungeonId,
            dungeonName = dungeon.name,
            timeLimit = dungeon.timeLimit,
            difficulty = dungeon.difficulty,
            players = {},
            totalEncounters = #dungeon.encounters,
            currentEncounter = instance.encounterIndex,
            remaining = math.max(0, remaining),
        }))
    else
        -- Instance ended while offline
        player:teleportTo(player:getTown():getTemplePosition())
        player:setStorageValue(Storage.Dungeon.CurrentInstance, 0)
        player:setStorageValue(Storage.Dungeon.CurrentDungeonId, 0)
        player:sendTextMessage(MESSAGE_INFO_DESCR, "Your dungeon instance has ended.")
    end
end

-- Death handler
function DungeonManager.onPlayerDeath(player)
    local instance, key = DungeonManager.getInstanceByPlayer(player)
    if not instance then return true end -- normal death

    local dungeon = DungeonConfig[instance.dungeonId]
    local slot = dungeon.slots[instance.slotIndex]

    -- Prevent actual death: heal and teleport to graveyard
    player:addHealth(math.floor(player:getMaxHealth() * 0.5))
    player:addMana(math.floor(player:getMaxMana() * 0.5))
    player:teleportTo(slot.graveyard)
    slot.graveyard:sendMagicEffect(CONST_ME_TELEPORT)

    instance.deaths = (instance.deaths or 0) + 1

    player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "You have been resurrected at the dungeon graveyard.")

    -- Broadcast death count
    DungeonManager.broadcastToInstance(key, {
        action = "death_update",
        deaths = instance.deaths,
    })

    -- Check party wipe
    DungeonManager.checkPartyWipe(key)

    return false -- prevent actual death
end

function DungeonManager.checkPartyWipe(key)
    local instance = DungeonManager.activeInstances[key]
    if not instance then return end

    -- In this simplified version, we don't track simultaneous deaths
    -- A full implementation would check if all players are dead at the same time
end

-- Build dungeon list for a player (for the finder UI)
function DungeonManager.getDungeonList(player)
    local list = {}
    for id, dungeon in pairs(DungeonConfig) do
        local lockout = DungeonManager.hasLockout(player, id, "daily")
        local slotsInUse = 0
        local totalSlots = 0
        for slotIndex, _ in pairs(dungeon.slots) do
            totalSlots = totalSlots + 1
            local k = DungeonManager.slotKey(id, slotIndex)
            if DungeonManager.activeInstances[k] then
                slotsInUse = slotsInUse + 1
            end
        end

        -- Build encounter chain for UI
        local encounterChain = {}
        for i, enc in ipairs(dungeon.encounters) do
            local outfits = {}
            if enc.spawns then
                for _, sp in ipairs(enc.spawns) do
                    local mType = MonsterType(sp.monster)
                    if mType then
                        local o = mType:getOutfit()
                        table.insert(outfits, {type = o.lookType or 0, head = o.lookHead or 0, body = o.lookBody or 0, legs = o.lookLegs or 0, feet = o.lookFeet or 0})
                    end
                end
            end
            table.insert(encounterChain, {
                name = enc.name,
                type = enc.type,
                outfits = outfits,
            })
        end

        -- Build boss loot list for UI (client item IDs)
        local bossLoot = {}
        for _, enc in ipairs(dungeon.encounters) do
            if enc.type == "boss" and enc.loot then
                for _, l in ipairs(enc.loot) do
                    local it = ItemType(l.itemId)
                    local clientId = it and it:getClientId() or l.itemId
                    table.insert(bossLoot, {
                        itemId = l.itemId,
                        clientId = clientId,
                        name = l.name,
                        chance = l.chance,
                    })
                end
            end
        end

        -- Get a representative creature outfit for the sidebar
        local previewOutfit = nil
        for _, enc in ipairs(dungeon.encounters) do
            if enc.type == "boss" and enc.spawns and enc.spawns[1] then
                local mType = MonsterType(enc.spawns[1].monster)
                if mType then
                    local o = mType:getOutfit()
                    previewOutfit = {type = o.lookType or 0, head = o.lookHead or 0, body = o.lookBody or 0, legs = o.lookLegs or 0, feet = o.lookFeet or 0}
                end
                break
            end
        end

        table.insert(list, {
            id = id,
            name = dungeon.name,
            description = dungeon.description,
            minLevel = dungeon.minLevel,
            maxPlayers = dungeon.maxPlayers,
            minPlayers = dungeon.minPlayers or 1,
            difficulty = dungeon.difficulty,
            timeLimit = dungeon.timeLimit,
            encounterCount = #dungeon.encounters,
            encounters = encounterChain,
            bossLoot = bossLoot,
            previewOutfit = previewOutfit,
            lockout = lockout,
            slotsAvailable = totalSlots - slotsInUse,
            totalSlots = totalSlots,
            queueOpen = (totalSlots - slotsInUse) > 0 and not lockout,
            rewards = dungeon.completionRewards,
        })
    end
    table.sort(list, function(a, b) return a.minLevel < b.minLevel end)
    return list
end
