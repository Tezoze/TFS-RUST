-- Force refresh quest log
-- Use in-game: /refreshquests

local refreshQuests = TalkAction("/refreshquests")

function refreshQuests.onSay(player, words, param)
    if not player:getGroup():getAccess() then
        return true
    end

    -- Force send quest log
    player:sendQuestLog()
    player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "Quest log has been refreshed!")
    
    return false
end

refreshQuests:separator(" ")
refreshQuests:register()
