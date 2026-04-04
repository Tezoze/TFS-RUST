-- Debug quest log system
-- Use in-game: /debugquest

local debugQuest = TalkAction("/debugquest")

function debugQuest.onSay(player, words, param)
    if not player:getGroup():getAccess() then
        return true
    end

    -- Get all quests
    local allQuests = Game.getQuests()
    player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "=== All Registered Quests ===")
    player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "Total: " .. #allQuests)
    
    -- Check our specific quests
    local childrenQuest = nil
    local wrathQuest = nil
    
    for _, quest in ipairs(allQuests) do
        if quest.name == "Children of the Revolution" then
            childrenQuest = quest
            player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "Found Children quest, ID: " .. quest.id)
            player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "  storageId: " .. quest.storageId .. ", storageValue: " .. quest.storageValue)
            player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "  isStarted: " .. tostring(quest:isStarted(player)))
            player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "  Player storage: " .. player:getStorageValue(quest.storageId))
        elseif quest.name == "Wrath of the Emperor" then
            wrathQuest = quest
            player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "Found Wrath quest, ID: " .. quest.id)
            player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "  storageId: " .. quest.storageId .. ", storageValue: " .. quest.storageValue)
            player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "  isStarted: " .. tostring(quest:isStarted(player)))
            player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "  Player storage: " .. player:getStorageValue(quest.storageId))
        end
    end
    
    -- Get player's quests
    local playerQuests = player:getQuests()
    player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "=== Player's Active Quests ===")
    player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "Total: " .. #playerQuests)
    
    for _, quest in ipairs(playerQuests) do
        player:sendTextMessage(MESSAGE_EVENT_ADVANCE, quest.name)
    end

    return false
end

debugQuest:separator(" ")
debugQuest:register()
