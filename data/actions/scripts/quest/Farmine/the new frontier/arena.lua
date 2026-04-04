local config = {
    bosses = {
        {'Baron Brute', 'The Axeorcist'},
        {'Menace', 'Fatality'},
        {'Incineron', 'Coldheart'},
        {'Dreadwing', 'Doomhowl'},
        {'Haunter', 'The Dreadorian'},
        {'Rocko', 'Tremorak'},
        {'Tirecz'}
    },
    teleportPositions = {
        Position(33059, 31032, 3),
        Position(33057, 31034, 3)
    },
    positions = {
        -- other bosses
        Position(33065, 31035, 3),
        Position(33068, 31034, 3),
        -- first 2 bosses
        Position(33065, 31033, 3),
        Position(33066, 31037, 3)
    },
    centerPosition = Position(33063, 31034, 3),
    exitPosition = Position(33049, 31017, 2),
    range = {x = 10, y = 10},
    storage = Storage.TheNewFrontier.Mission09
}

-- ID to track the kickoff timer
if not NEW_FRONTIER_ARENA_EVENT then
    NEW_FRONTIER_ARENA_EVENT = 0
end

local function clearArena()
    local spectators = Game.getSpectators(config.centerPosition, false, false, config.range.x, config.range.x, config.range.y, config.range.y)
    for i = 1, #spectators do
        local spectator = spectators[i]
        if spectator:isPlayer() then
            spectator:teleportTo(config.exitPosition)
            config.exitPosition:sendMagicEffect(CONST_ME_TELEPORT)
            spectator:sendTextMessage(MESSAGE_STATUS_SMALL, 'Time is up!')
        else
            spectator:remove()
        end
    end
    -- Reset storage so others can enter
    Game.setStorageValue(config.storage, -1)
end

local function summonBoss(name, position)
    -- OPTIMIZATION: Don't summon if the team died (room is empty)
    local spectators = Game.getSpectators(config.centerPosition, false, true, config.range.x, config.range.x, config.range.y, config.range.y)
    if #spectators == 0 then
        return
    end

    Game.createMonster(name, position)
    position:sendMagicEffect(CONST_ME_TELEPORT)
end

function onUse(player, item, fromPosition, target, toPosition, isHotkey)
    -- 1. IDENTIFY PLAYERS
    local player1 = Tile(Position({x = 33080, y = 31014, z = 2})):getTopCreature()
    local player2 = Tile(Position({x = 33081, y = 31014, z = 2})):getTopCreature()

    -- 2. STRICT CHECK: Both players must exist
    if not (player1 and player1:isPlayer()) then
        player:sendTextMessage(MESSAGE_STATUS_SMALL, 'You need a partner to enter the arena.')
        return true
    end

    if not (player2 and player2:isPlayer()) then
        player:sendTextMessage(MESSAGE_STATUS_SMALL, 'You need a partner to enter the arena.')
        return true
    end

    -- 3. QUEST STATUS CHECK (Both must have Mission09 started but not completed)
    if player1:getStorageValue(Storage.TheNewFrontier.Mission09) ~= 1 then
        if player1:getStorageValue(Storage.TheNewFrontier.Mission09) >= 2 then
            player1:sendTextMessage(MESSAGE_STATUS_SMALL, 'You have already finished this battle.')
        else
            player1:sendTextMessage(MESSAGE_STATUS_SMALL, 'You are not ready for this challenge yet.')
        end
        return true
    end

    if player2:getStorageValue(Storage.TheNewFrontier.Mission09) ~= 1 then
        if player2:getStorageValue(Storage.TheNewFrontier.Mission09) >= 2 then
            player2:sendTextMessage(MESSAGE_STATUS_SMALL, player2:getName() .. ' has already finished this battle.')
        else
            player2:sendTextMessage(MESSAGE_STATUS_SMALL, player2:getName() .. ' is not ready for this challenge yet.')
        end
        return true
    end

    -- 4. ARENA BUSY CHECK (With Self-Healing)
    -- If storage says "Busy" (1), check if anyone is ACTUALLY inside.
    if Game.getStorageValue(config.storage) == 1 then
        local spectators = Game.getSpectators(config.centerPosition, false, true, config.range.x, config.range.x, config.range.y, config.range.y)
        if #spectators > 0 then
            player:sendTextMessage(MESSAGE_STATUS_SMALL, 'The arena is currently in use.')
            return true
        else
            -- Room is empty but storage was stuck? Reset it immediately.
            -- This allows "Instant Retry" if the previous team died.
            Game.setStorageValue(config.storage, -1)
            stopEvent(NEW_FRONTIER_ARENA_EVENT)
        end
    end

    -- 5. START BATTLE
    Game.setStorageValue(config.storage, 1)
    
    -- Stop previous timer if it exists
    stopEvent(NEW_FRONTIER_ARENA_EVENT)
    -- Schedule the 30-minute kick
    NEW_FRONTIER_ARENA_EVENT = addEvent(clearArena, 30 * 60 * 1000)

    -- Register the kill event for both players (in case they logged in before this quest stage)
    player1:registerEvent("NewFrontierTirecz")
    player2:registerEvent("NewFrontierTirecz")

    -- Teleport BOTH players
    player1:teleportTo(config.teleportPositions[1])
    player1:getPosition():sendMagicEffect(CONST_ME_TELEPORT)
    
    player2:teleportTo(config.teleportPositions[2])
    player2:getPosition():sendMagicEffect(CONST_ME_TELEPORT)

    -- Summon Loop
    for i = 1, #config.bosses do
        for j = 1, #config.bosses[i] do
            addEvent(summonBoss, (i - 1) * 90 * 1000, config.bosses[i][j], config.positions[j + (i == 1 and 2 or 0)])
        end
    end
    
    return true
end