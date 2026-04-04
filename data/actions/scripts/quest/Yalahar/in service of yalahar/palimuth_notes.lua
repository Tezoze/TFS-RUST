function onUse(player, item, fromPosition, target, toPosition, isHotkey)
    -- Check if player is at the correct quest stage
    if player:getStorageValue(Storage.InServiceofYalahar.Questline) ~= 16 then
        player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "You can't read these notes right now.")
        return true
    end

    -- Check if notes haven't been read yet
    if player:getStorageValue(Storage.InServiceofYalahar.NotesPalimuth) ~= 0 then
        player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "You have already read these notes.")
        return true
    end

    -- Display the notes content
    player:showTextDialog(item, "Dear friend,\n\nI hope you understand that I can't tell you everything openly. There are ears and eyes everywhere in this city.\n\nThe situation in Yalahar is more complex than it appears. The Yalahari claim to work for the good of the city, but I have my doubts about their true intentions.\n\nSome of their recent actions seem to benefit only themselves rather than the citizens. The people are suffering while they pursue their own agenda.\n\nI trust you will make the right decisions when the time comes. Remember that the fate of many innocent people may depend on your choices.\n\nBe careful who you trust.\n\n- Palimuth")

    -- Set storage to indicate notes have been read
    player:setStorageValue(Storage.InServiceofYalahar.NotesPalimuth, 1)
    player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "You have read Palimuth's notes. You should talk to him again.")

    return true
end

