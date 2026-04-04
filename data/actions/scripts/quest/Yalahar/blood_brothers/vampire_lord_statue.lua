-- Blood Brothers Quest - Vampire Lord Statue (Arthei)
-- Mission 10: Use Ghost's Tear on the statue twice
--
-- First use: Statue transforms (9241 -> 9242) for 1 minute, displays "WHAT DO YOU THINK YOU'RE DOING MORTAL"
-- Second use: Teleports player to Arthei boss room with 10-minute timer
--
-- CONFIGURABLE VALUES (update these with your specific positions and IDs):
-- - ARTHEI_ROOM_POSITION: Where to teleport player for boss fight
-- - STATUE_POSITION: Position of the statue (for timeout teleport)
-- - STATUE_NORMAL_ID: Normal statue item ID (default: 9241)
-- - STATUE_TRANSFORM_ITEM: Transformed statue item ID (default: 9242)

local STATUE_TRANSFORM_ITEM = 9242  -- Item ID for transformed statue
local TRANSFORM_DURATION = 60 * 1000  -- 1 minute in milliseconds
local BOSS_FIGHT_DURATION = 10 * 60 * 1000  -- 10 minutes in milliseconds

local ARTHEI_ROOM_POSITION = Position(32953, 31444, 1)  -- Position to teleport player to Arthei's chamber (configurable)
local STATUE_POSITION = Position(32953, 31440, 2)  -- Position of the statue (configurable)

local STATUE_NORMAL_ID = 9241  -- Normal Vampire Lord Statue item ID
local STATUE_TRANSFORM_ITEM = 9242  -- Item ID for transformed statue

function onUse(player, item, fromPosition, target, toPosition, isHotkey)
    -- Only allow using Ghost's Tear on the Vampire Lord Statue (normal or transformed)
    local targetId = target and target.getId and target:getId() or 0
    if targetId ~= 9241 and targetId ~= 9242 then
        player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "You can only use this tear on the Vampire Lord Statue.")
        return true
    end

    local tearItem = item

    -- Check if player is on Mission 10
    local mission10Value = player:getStorageValue(Storage.BloodBrothers.Mission10)
    if mission10Value ~= 1 then
        player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "You have no reason to use this tear on the statue right now.")
        return true
    end

    -- Check if this is the first or second use
    local statueUses = player:getStorageValue(Storage.BloodBrothers.ArtheiStatueUses)
    if statueUses == -1 then
        statueUses = 0 -- Reset corrupted storage value
        player:setStorageValue(Storage.BloodBrothers.ArtheiStatueUses, 0)
    elseif statueUses == nil then
        statueUses = 0
    end

    if statueUses == 0 then
        -- First use: Transform statue and show message

        -- Transform the statue temporarily
        local statueTile = Tile(toPosition)
        if statueTile then
            local statueItem = statueTile:getItemById(STATUE_NORMAL_ID)
            if statueItem then
                -- Remove old statue and create transformed one
                statueItem:transform(STATUE_TRANSFORM_ITEM)

                -- Schedule restoration after 1 minute
                addEvent(function()
                    local currentStatue = statueTile:getItemById(STATUE_TRANSFORM_ITEM)
                    if currentStatue then
                        currentStatue:transform(STATUE_NORMAL_ID)  -- Restore to normal statue
                    end
                end, TRANSFORM_DURATION)

                -- Display monster message
                player:say("WHAT DO YOU THINK YOU'RE DOING MORTAL", TALKTYPE_MONSTER_SAY)

                -- Update usage count
                player:setStorageValue(Storage.BloodBrothers.ArtheiStatueUses, 1)

                -- Keep the tear in inventory for second use
            end
        end

    elseif statueUses == 1 then
        -- Second use: Teleport to boss room

        -- Teleport player to boss room
        player:teleportTo(ARTHEI_ROOM_POSITION)
        player:getPosition():sendMagicEffect(CONST_ME_TELEPORT)

        -- Spawn Arthei and 2 Vampires
        Game.createMonster("Arthei", Position(32953, 31442, 1))
        Game.createMonster("Vampire", Position(32951, 31441, 1))
        Game.createMonster("Vampire", Position(32955, 31441, 1))

        -- Set boss fight timer
        player:setStorageValue(Storage.BloodBrothers.ArtheiFightStart, os.time())

        -- Schedule boss fight timeout
        addEvent(function()
            if player then
                local fightStart = player:getStorageValue(Storage.BloodBrothers.ArtheiFightStart)
                if fightStart and (os.time() - fightStart) >= 600 then  -- 10 minutes
                    -- Check if Arthei is still alive
                    local artheiAlive = false
                    local spectators = Game.getSpectators(ARTHEI_ROOM_POSITION, false, false, 10, 10, 10, 10)
                    for _, creature in ipairs(spectators) do
                        if creature:getName() == "Arthei" then
                            artheiAlive = true
                            break
                        end
                    end

                    if artheiAlive then
                        -- Teleport player out if fight timed out
                        player:teleportTo(STATUE_POSITION)
                        player:getPosition():sendMagicEffect(CONST_ME_TELEPORT)
                        player:setStorageValue(Storage.BloodBrothers.ArtheiFightStart, -1)
                    end
                end
            end
        end, BOSS_FIGHT_DURATION)

        -- Remove the tear from inventory
        tearItem:remove(1)

        -- Reset usage count
        player:setStorageValue(Storage.BloodBrothers.ArtheiStatueUses, 0)

    end

    return true
end
