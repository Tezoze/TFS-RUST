function onUse(player, item, fromPosition, target, toPosition, isHotkey)
    -- Check if player is at the correct quest stage
    if player:getStorageValue(Storage.InServiceofYalahar.Questline) ~= 18 then
        player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "You can't read this manifesto right now.")
        return true
    end

    -- Get current notes read count
    local notesRead = player:getStorageValue(Storage.InServiceofYalahar.NotesAzerus) or 0
    
    -- Check if all notes have been read
    if notesRead >= 2 then
        player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "You have already read all the manifesto notes.")
        return true
    end

    -- Display the manifesto content
    player:showTextDialog(item, "YALAHARI MANIFESTO\n\nWe, the Yalahari, are the rightful rulers of this great city. For generations, we have provided for its citizens and protected them from the chaos of the outside world.\n\nThe current troubles are temporary setbacks that require decisive action. Those who oppose our authority only bring more suffering to the innocent people of Yalahar.\n\nWe seek capable individuals who understand that order must be maintained at all costs. The weak and indecisive have no place in our new vision for the city.\n\nJoin us, and you will be rewarded with power, wealth, and status befitting your abilities. Serve us well, and rise through our ranks.\n\nOppose us, and face the consequences of your poor judgment.\n\nThe choice is yours, but choose wisely.\n\n- The Yalahari Council")

    -- Increment notes read count
    notesRead = notesRead + 1
    player:setStorageValue(Storage.InServiceofYalahar.NotesAzerus, notesRead)
    
    -- Check if all notes have been read
    if notesRead >= 2 then
        player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "You have read all parts of the Yalahari manifesto. You should talk to Azerus again.")
    else
        player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "You have read part " .. notesRead .. " of the Yalahari manifesto. Find and read the remaining parts.")
    end

    return true
end
