--[[
    Dungeon System — Server Script
    Handles: Extended Opcode 210, TalkAction, CreatureEvents
    Protocol: 210
]]

local OPCODE = 210

-------------------------------------------------
-- Extended Opcode Handler
-------------------------------------------------
local function handleDungeonOpcode(player, buffer)
    local success, data = pcall(json.decode, buffer)
    if not success or not data then return end

    local action = data.action

    if action == "list" then
        local list = DungeonManager.getDungeonList(player)
        local inDungeon = DungeonManager.isInDungeon(player)
        player:sendExtendedOpcode(OPCODE, json.encode({
            action = "dungeon_list",
            dungeons = list,
            inDungeon = inDungeon,
        }))

    elseif action == "enter" then
        local dungeonId = tonumber(data.id)
        if not dungeonId then return end
        DungeonManager.enterDungeon(player, dungeonId)

    elseif action == "leave_dungeon" then
        if DungeonManager.isInDungeon(player) then
            DungeonManager.leaveDungeon(player)
            player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "You have left the dungeon.")
        end

    elseif action == "get_status" then
        local instance, key = DungeonManager.getInstanceByPlayer(player)
        if instance then
            local dungeon = DungeonConfig[instance.dungeonId]
            local elapsed = os.time() - instance.startTime
            local remaining = math.max(0, (dungeon.timeLimit or 3600) - elapsed)
            local encounter = dungeon.encounters[instance.encounterIndex]
            player:sendExtendedOpcode(OPCODE, json.encode({
                action = "status_update",
                dungeonName = dungeon.name,
                difficulty = dungeon.difficulty,
                remaining = remaining,
                encounterIndex = instance.encounterIndex,
                totalEncounters = #dungeon.encounters,
                encounterName = encounter and encounter.name or "Exploring...",
                encounterType = encounter and encounter.type or "none",
                phase = instance.currentPhase,
                totalPhases = encounter and encounter.phases and #encounter.phases or 1,
                deaths = instance.deaths,
                score = DungeonManager.calculateScore(instance.deaths, elapsed, dungeon.timeLimit),
            }))
        end
    end
end

-------------------------------------------------
-- Login Event: Register dungeon events on player
-------------------------------------------------
local login = CreatureEvent("DungeonPlayerLogin")
function login.onLogin(player)
    player:registerEvent("DungeonOpcode")
    player:registerEvent("DungeonPlayerDeath")
    player:registerEvent("DungeonPlayerLogout")
    DungeonManager.onPlayerLogin(player)
    return true
end
login:register()

-------------------------------------------------
-- Extended Opcode Event
-------------------------------------------------
local op = CreatureEvent("DungeonOpcode")
function op.onExtendedOpcode(player, opcode, buffer)
    if opcode ~= OPCODE then return end
    handleDungeonOpcode(player, buffer)
end
op:type("extendedopcode")
op:register()

-------------------------------------------------
-- Death Event: Override death inside dungeons
-------------------------------------------------
local death = CreatureEvent("DungeonPlayerDeath")
function death.onPrepareDeath(creature, killer)
    local player = creature:getPlayer()
    if not player then return true end
    if DungeonManager.isInDungeon(player) then
        return DungeonManager.onPlayerDeath(player)
    end
    return true
end
death:type("preparedeath")
death:register()

-------------------------------------------------
-- Logout Event: Handle disconnect grace period
-------------------------------------------------
local logout = CreatureEvent("DungeonPlayerLogout")
function logout.onLogout(player)
    if DungeonManager.isInDungeon(player) then
        local instance, key = DungeonManager.getInstanceByPlayer(player)
        if instance then
            for _, pd in ipairs(instance.players) do
                if pd.id == player:getId() then
                    pd.connected = false
                    break
                end
            end
            -- Grace timer
            local playerId = player:getId()
            addEvent(function()
                local inst = DungeonManager.activeInstances[key]
                if not inst then return end
                for _, pd in ipairs(inst.players) do
                    if pd.id == playerId and not pd.connected then
                        -- Player didn't reconnect, remove them
                        local p = Player(playerId)
                        if p then
                            DungeonManager.removePlayerFromInstance(p, key)
                        else
                            -- Player still offline, clear their mapping
                            DungeonManager.playerToSlot[playerId] = nil
                        end
                    end
                end
            end, DungeonManager.DISCONNECT_GRACE * 1000)
        end
    end
    return true
end
logout:type("logout")
logout:register()

-------------------------------------------------
-- TalkAction: !dungeon
-------------------------------------------------
local cmd = TalkAction("!dungeon")
function cmd.onSay(player, words, param, type)
    if param == "leave" then
        if DungeonManager.isInDungeon(player) then
            DungeonManager.leaveDungeon(player)
            player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "You have left the dungeon.")
        else
            player:sendTextMessage(MESSAGE_STATUS_SMALL, "You are not in a dungeon.")
        end
        return false
    end

    -- Open the dungeon finder UI
    local list = DungeonManager.getDungeonList(player)
    local inDungeon = DungeonManager.isInDungeon(player)
    player:sendExtendedOpcode(OPCODE, json.encode({
        action = "open",
        dungeons = list,
        inDungeon = inDungeon,
    }))
    return false
end
cmd:separator(" ")
cmd:register()

-------------------------------------------------
-- GlobalEvent: Periodic timer sync
-------------------------------------------------
local cleanup = GlobalEvent("DungeonTimerSync")
function cleanup.onThink(interval)
    for key, instance in pairs(DungeonManager.activeInstances) do
        if not instance.completed then
            local dungeon = DungeonConfig[instance.dungeonId]
            local elapsed = os.time() - instance.startTime
            local timeLimit = dungeon.timeLimit or DungeonManager.MAX_TIME
            local remaining = math.max(0, timeLimit - elapsed)

            DungeonManager.broadcastToInstance(key, {
                action = "timer_sync",
                remaining = remaining,
            })
        end
    end
    return true
end
cleanup:interval(30000) -- Every 30 seconds
cleanup:register()
