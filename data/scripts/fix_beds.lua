-- Fix beds that have sleeping sprite but no sleeper
-- Usage: /fixbeds

local sleepingToEmpty = {
    -- Male sleeper IDs -> Empty bed IDs
    [7787] = 7811, [7788] = 7812, [7789] = 7813, [7790] = 7814,
    [7791] = 7815, [7792] = 7816, [7793] = 7817, [7794] = 7818,
    [7795] = 7819, [7796] = 7820, [7797] = 7821, [7798] = 7822,
    -- Female sleeper IDs -> Empty bed IDs
    [7799] = 7811, [7800] = 7812, [7801] = 7813, [7802] = 7814,
    [7803] = 7815, [7804] = 7816, [7805] = 7817, [7806] = 7818,
    [7807] = 7819, [7808] = 7820, [7809] = 7821, [7810] = 7822,
}

local fixBeds = TalkAction("/fixbeds")

function fixBeds.onSay(player, words, param)
    if player:getGroup():getAccess() == false then
        return false
    end

    local count = 0
    for _, house in pairs(Game.getHouses()) do
        for _, tile in pairs(house:getTiles()) do
            local bed = tile:getItemByType(ITEM_TYPE_BED)
            if bed then
                local bedId = bed:getId()
                local emptyId = sleepingToEmpty[bedId]
                if emptyId and bed:getAttribute(ITEM_ATTRIBUTE_SLEEPERGUID) == 0 then
                    bed:transform(emptyId)
                    count = count + 1
                end
            end
        end
    end

    player:sendTextMessage(MESSAGE_INFO_DESCR, "Fixed " .. count .. " beds.")
    return false
end

fixBeds:separator(" ")
fixBeds:register()
