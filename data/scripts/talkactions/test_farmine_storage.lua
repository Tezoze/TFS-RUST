-- Temporary test talkaction to check Farmine quest storage values
-- Use in-game: /checkfarmine

local testFarmineStorage = TalkAction("/checkfarmine")

function testFarmineStorage.onSay(player, words, param)
    if not player:getGroup():getAccess() then
        return true
    end

    player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "=== Children of the Revolution ===")
    player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "Questline: " .. player:getStorageValue(Storage.ChildrenoftheRevolution.Questline))
    player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "Mission00: " .. player:getStorageValue(Storage.ChildrenoftheRevolution.Mission00))
    player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "Mission01: " .. player:getStorageValue(Storage.ChildrenoftheRevolution.Mission01))
    player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "Mission02: " .. player:getStorageValue(Storage.ChildrenoftheRevolution.Mission02))
    player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "Mission03: " .. player:getStorageValue(Storage.ChildrenoftheRevolution.Mission03))
    player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "Mission04: " .. player:getStorageValue(Storage.ChildrenoftheRevolution.Mission04))
    player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "Mission05: " .. player:getStorageValue(Storage.ChildrenoftheRevolution.Mission05))

    player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "=== Wrath of the Emperor ===")
    player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "Questline: " .. player:getStorageValue(Storage.WrathoftheEmperor.Questline))
    player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "Mission01: " .. player:getStorageValue(Storage.WrathoftheEmperor.Mission01))
    player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "Mission02: " .. player:getStorageValue(Storage.WrathoftheEmperor.Mission02))
    player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "Mission03: " .. player:getStorageValue(Storage.WrathoftheEmperor.Mission03))
    player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "Mission04: " .. player:getStorageValue(Storage.WrathoftheEmperor.Mission04))
    player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "Mission05: " .. player:getStorageValue(Storage.WrathoftheEmperor.Mission05))
    player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "Mission06: " .. player:getStorageValue(Storage.WrathoftheEmperor.Mission06))
    player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "Mission07: " .. player:getStorageValue(Storage.WrathoftheEmperor.Mission07))
    player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "Mission08: " .. player:getStorageValue(Storage.WrathoftheEmperor.Mission08))
    player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "Mission09: " .. player:getStorageValue(Storage.WrathoftheEmperor.Mission09))
    player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "Mission10: " .. player:getStorageValue(Storage.WrathoftheEmperor.Mission10))
    player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "Mission11: " .. player:getStorageValue(Storage.WrathoftheEmperor.Mission11))
    player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "BossStatus: " .. player:getStorageValue(Storage.WrathoftheEmperor.BossStatus))

    return false
end

testFarmineStorage:separator(" ")
testFarmineStorage:register()
