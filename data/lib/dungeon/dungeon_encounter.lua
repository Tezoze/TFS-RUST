--[[
    Dungeon System — Encounter Engine
    Drives trash pack and boss encounter progression.
]]

DungeonEncounter = {}

-- Encounter states
DungeonEncounter.STATE_IDLE = 0
DungeonEncounter.STATE_ACTIVE = 1
DungeonEncounter.STATE_CLEARED = 2

function DungeonEncounter.startEncounter(key, encounterIndex)
    local instance = DungeonManager.activeInstances[key]
    if not instance then return end

    local dungeon = DungeonConfig[instance.dungeonId]
    local encounter = dungeon.encounters[encounterIndex]
    if not encounter then return end

    instance.encounterIndex = encounterIndex
    instance.encounterActive = true
    instance.currentPhase = 1
    instance.encounterCreatures = {}

    -- Spawn monsters
    for _, spawnDef in ipairs(encounter.spawns) do
        for i = 1, (spawnDef.count or 1) do
            local pos = DungeonManager.getAbsolutePos(instance.slotIndex, instance.dungeonId, spawnDef.pos)
            if pos then
                -- Offset each spawn slightly to avoid stacking
                local spawnPos = Position(pos.x + math.random(-2, 2), pos.y + math.random(-2, 2), pos.z)
                local monster = Game.createMonster(spawnDef.monster, spawnPos, false, true)
                if monster then
                    table.insert(instance.spawnedCreatures, monster:getId())
                    table.insert(instance.encounterCreatures, monster:getId())

                    -- Register death callback
                    local deathEvent = CreatureEvent("dungeon_mob_death_" .. monster:getId())
                    deathEvent:type("death")
                    deathEvent:onDeath(function(creature, corpse, killer, mostDamageKiller, lastHitUnjustified, mostDamageUnjustified)
                        DungeonManager.onCreatureDeath(key, creature:getId())
                    end)
                    deathEvent:register()
                    monster:registerEvent("dungeon_mob_death_" .. monster:getId())
                end
            end
        end
    end

    -- Broadcast encounter start
    DungeonManager.broadcastToInstance(key, {
        action = "encounter_update",
        encounter = encounter.name,
        type = encounter.type,
        phase = 1,
        totalPhases = encounter.phases and #encounter.phases or 1,
        encounterIndex = encounterIndex,
        totalEncounters = #dungeon.encounters,
    })

    print("[Dungeon] Started encounter " .. encounterIndex .. ": " .. encounter.name .. " in slot " .. key)
end

function DungeonEncounter.checkEncounterCleared(key)
    local instance = DungeonManager.activeInstances[key]
    if not instance or not instance.encounterActive then return end

    -- Check if all encounter creatures are dead
    if instance.encounterCreatures then
        for _, cid in ipairs(instance.encounterCreatures) do
            local creature = Creature(cid)
            if creature and creature:getHealth() > 0 then
                return -- Still alive
            end
        end
    end

    -- Encounter cleared
    instance.encounterActive = false
    instance.encounterCreatures = {}

    local dungeon = DungeonConfig[instance.dungeonId]
    local encounter = dungeon.encounters[instance.encounterIndex]

    -- Distribute loot if boss
    if encounter and encounter.type == "boss" and encounter.loot then
        DungeonLoot.distributeLoot(key, encounter)
    end

    -- Broadcast cleared
    DungeonManager.broadcastToInstance(key, {
        action = "encounter_cleared",
        encounter = encounter and encounter.name or "Unknown",
        encounterIndex = instance.encounterIndex,
        totalEncounters = #dungeon.encounters,
    })

    -- Check if dungeon is complete
    if instance.encounterIndex >= #dungeon.encounters then
        addEvent(DungeonManager.completeDungeon, 3000, key)
        return
    end

    -- Trigger next encounter if kill_previous
    local nextEncounter = dungeon.encounters[instance.encounterIndex + 1]
    if nextEncounter and nextEncounter.triggerType == "kill_previous" then
        addEvent(DungeonEncounter.startEncounter, 3000, key, instance.encounterIndex + 1)
    else
        -- Resume proximity checks
        addEvent(DungeonManager.checkProximityTriggers, 2000, key)
    end
end

-- Boss phase transition (called from health change monitoring)
function DungeonEncounter.checkPhaseTransition(key, creature)
    local instance = DungeonManager.activeInstances[key]
    if not instance or not instance.encounterActive then return end

    local dungeon = DungeonConfig[instance.dungeonId]
    local encounter = dungeon.encounters[instance.encounterIndex]
    if not encounter or encounter.type ~= "boss" or not encounter.phases then return end

    local hpPercent = (creature:getHealth() / creature:getMaxHealth()) * 100

    for phaseNum, phase in ipairs(encounter.phases) do
        if phaseNum > instance.currentPhase and hpPercent <= phase.hpPercent then
            instance.currentPhase = phaseNum
            DungeonEncounter.executePhase(key, encounter, phase, phaseNum)
            break
        end
    end
end

function DungeonEncounter.executePhase(key, encounter, phase, phaseNum)
    local instance = DungeonManager.activeInstances[key]
    if not instance then return end

    for _, mechanic in ipairs(phase.mechanics or {}) do
        if mechanic == "summon_adds" and phase.adds then
            for _, addDef in ipairs(phase.adds) do
                for i = 1, (addDef.count or 1) do
                    local bossPos = DungeonManager.getAbsolutePos(
                        instance.slotIndex, instance.dungeonId,
                        encounter.spawns[1].pos
                    )
                    if bossPos then
                        local spawnPos = Position(bossPos.x + math.random(-3, 3), bossPos.y + math.random(-3, 3), bossPos.z)
                        local monster = Game.createMonster(addDef.monster, spawnPos, false, true)
                        if monster then
                            table.insert(instance.spawnedCreatures, monster:getId())
                            table.insert(instance.encounterCreatures, monster:getId())
                            local deathEvent = CreatureEvent("dungeon_add_death_" .. monster:getId())
                            deathEvent:type("death")
                            deathEvent:onDeath(function(creature, corpse, killer, mostDamageKiller, lastHitUnjustified, mostDamageUnjustified)
                                DungeonManager.onCreatureDeath(key, creature:getId())
                            end)
                            deathEvent:register()
                            monster:registerEvent("dungeon_add_death_" .. monster:getId())
                        end
                    end
                end
            end
        elseif mechanic == "enrage" and phase.enrage then
            -- Apply enrage effect to boss (visual only for now, damage scaling needs C++ or condition)
            local bossPos = DungeonManager.getAbsolutePos(
                instance.slotIndex, instance.dungeonId,
                encounter.spawns[1].pos
            )
            if bossPos then
                bossPos:sendMagicEffect(CONST_ME_HITBYFIRE)
            end
        elseif mechanic == "area_denial" then
            -- Spawn fire fields around the boss area
            local bossPos = DungeonManager.getAbsolutePos(
                instance.slotIndex, instance.dungeonId,
                encounter.spawns[1].pos
            )
            if bossPos then
                for i = 1, 5 do
                    local fieldPos = Position(bossPos.x + math.random(-4, 4), bossPos.y + math.random(-4, 4), bossPos.z)
                    local tile = Tile(fieldPos)
                    if tile and tile:isWalkable() then
                        Game.createItem(1492, 1, fieldPos) -- Fire field
                    end
                end
            end
        end
    end

    -- Broadcast phase change
    DungeonManager.broadcastToInstance(key, {
        action = "encounter_update",
        encounter = encounter.name,
        type = "boss",
        phase = phaseNum,
        totalPhases = #encounter.phases,
        encounterIndex = instance.encounterIndex,
        totalEncounters = #DungeonConfig[instance.dungeonId].encounters,
    })
end
