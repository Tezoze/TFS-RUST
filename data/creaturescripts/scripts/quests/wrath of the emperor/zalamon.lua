local bossForms = {
    ['snake god essence'] = {
        text = 'IT\'S NOT THAT EASY MORTALS! FEEL THE POWER OF THE GOD!',
        newForm = 'snake thing'
    },
    ['snake thing'] = {
        text = 'NOOO! NOW YOU HERETICS WILL FACE MY GODLY WRATH!',
        newForm = 'lizard abomination'
    },
    ['lizard abomination'] = {
        text = 'YOU ... WILL ... PAY WITH ETERNITY ... OF AGONY!',
        newForm = 'mutated zalamon'
    }
}

-- Adding duplicate prevention for Zalamon kills
local recent_zalamon_kills = {}
local last_arena_reset = 0

function onKill(creature, target)
    local player = creature:getPlayer()
    if not player then
        return true
    end

    if not target then
        return true
    end

    -- Check if arena has been reset since last kill and clear table if needed
    local currentArenaStatus = Game.getStorageValue(Storage.WrathoftheEmperor.Mission11) or 0
    if currentArenaStatus == 1 and last_arena_reset ~= currentArenaStatus then
        -- Arena just started/reset, clear the duplicate prevention table
        recent_zalamon_kills = {}
        last_arena_reset = currentArenaStatus
    elseif currentArenaStatus == 0 then
        last_arena_reset = 0
    end

    -- Adding duplicate prevention for Zalamon kills
    local targetId = target:getId()
    local playerGuid = player:getGuid()

    if not recent_zalamon_kills[playerGuid] then
        recent_zalamon_kills[playerGuid] = {}
    end

    if recent_zalamon_kills[playerGuid][targetId] then
        return true
    end

    recent_zalamon_kills[playerGuid][targetId] = true

    addEvent(function()
        if recent_zalamon_kills[playerGuid] then
            recent_zalamon_kills[playerGuid][targetId] = nil
        end
    end, 3000)

    if target:getName():lower() == 'mutated zalamon' then
        return true
    end

    local name = target:getName():lower()
    local bossConfig  = bossForms[name]
    if not bossConfig then
        return true
    end

    local found = false
    local spectators = Game.getSpectators(target:getPosition(), false, true, 5, 5, 5, 5)
    for k, v in ipairs(spectators) do
        if v and v.getName and v:getName():lower() == bossConfig.newForm then
            found = true
            break
        end
    end

    if not found then
        print("DEBUG: Spawning " .. bossConfig.newForm .. " at position " .. target:getPosition().x .. ", " .. target:getPosition().y .. ", " .. target:getPosition().z)
        local monster = Game.createMonster(bossConfig.newForm, target:getPosition(), false, true)
        if monster then
            if player then
                player:say(bossConfig.text, TALKTYPE_MONSTER_SAY)
            else
                -- Fallback if no player reference
                Game.broadcastMessage(bossConfig.text, MESSAGE_EVENT_ADVANCE)
            end
        else
            print("DEBUG: Failed to create monster " .. bossConfig.newForm)
        end
    else
        print("DEBUG: " .. bossConfig.newForm .. " already exists at position, not spawning")
    end
    return true
end
