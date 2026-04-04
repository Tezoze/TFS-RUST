-- Debug talkaction to check monster attributes
-- Usage: /monsterinfo <monster name>

local debugMonsterStats = TalkAction("/monsterinfo")

function debugMonsterStats.onSay(player, words, param)
    if not player:getGroup():getAccess() then
        return true
    end

    if param == "" then
        player:sendTextMessage(MESSAGE_STATUS_CONSOLE_BLUE, "Usage: /monsterinfo <monster name>")
        return false
    end

    local monsterType = MonsterType(param)
    if not monsterType then
        player:sendTextMessage(MESSAGE_STATUS_CONSOLE_BLUE, "Monster '" .. param .. "' not found.")
        return false
    end

    local name = monsterType:name()
    local health = monsterType:maxHealth()
    local armor = monsterType:armor()
    local defense = monsterType:defense()
    local experience = monsterType:experience()

    player:sendTextMessage(MESSAGE_STATUS_CONSOLE_BLUE, "=== Monster Info: " .. name .. " ===")
    player:sendTextMessage(MESSAGE_STATUS_CONSOLE_BLUE, "Health: " .. health)
    player:sendTextMessage(MESSAGE_STATUS_CONSOLE_BLUE, "Armor: " .. armor)
    player:sendTextMessage(MESSAGE_STATUS_CONSOLE_BLUE, "Defense: " .. defense)
    player:sendTextMessage(MESSAGE_STATUS_CONSOLE_BLUE, "Experience: " .. experience)

    -- Also print to console for server-side verification
    print("[DEBUG] Monster: " .. name)
    print("[DEBUG] - Health: " .. health)
    print("[DEBUG] - Armor: " .. armor)
    print("[DEBUG] - Defense: " .. defense)
    print("[DEBUG] - Experience: " .. experience)

    return false
end

debugMonsterStats:separator(" ")
debugMonsterStats:access(true)
debugMonsterStats:accountType(ACCOUNT_TYPE_GOD)
debugMonsterStats:register()
