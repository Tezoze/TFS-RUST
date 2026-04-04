-- Test if quest system is working
-- Use in-game: /testquests

local testQuests = TalkAction("/testquests")

function testQuests.onSay(player, words, param)
    if not player:getGroup():getAccess() then
        return true
    end

    local childrenQuestline = player:getStorageValue(Storage.ChildrenoftheRevolution.Questline)
    local wrathQuestline = player:getStorageValue(Storage.WrathoftheEmperor.Questline)
    
    player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "=== Quest System Test ===")
    player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "Children Questline Storage: " .. childrenQuestline)
    player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "Wrath Questline Storage: " .. wrathQuestline)
    
    -- Try to get quest objects
    local quests = Game.getQuests()
    player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "Total quests registered: " .. #quests)
    
    -- Check if our quests are registered
    local foundChildren = false
    local foundWrath = false
    
    for _, quest in ipairs(quests) do
        if quest:getName() == "Children of the Revolution" then
            foundChildren = true
            player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "Found: Children of the Revolution")
        end
        if quest:getName() == "Wrath of the Emperor" then
            foundWrath = true
            player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "Found: Wrath of the Emperor")
        end
    end
    
    if not foundChildren then
        player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "ERROR: Children of the Revolution NOT registered!")
    end
    if not foundWrath then
        player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "ERROR: Wrath of the Emperor NOT registered!")
    end

    return false
end

testQuests:separator(" ")
testQuests:register()
