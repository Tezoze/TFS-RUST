-- Talkaction to check wareId for items in your depot
function onSay(player, words, param)
    if not player:getGroup():getAccess() then
        return true
    end

    local depot = player:getDepotChest(player:getLastDepotId(), false)
    if not depot then
        player:sendTextMessage(MESSAGE_STATUS_CONSOLE_BLUE, "No depot found.")
        return false
    end

    local itemsWithWareId = 0
    local itemsWithoutWareId = 0
    local samples = {}

    local function checkContainer(container)
        for i = 0, container:getSize() - 1 do
            local item = container:getItem(i)
            if item then
                local itemType = ItemType(item:getId())
                local wareId = itemType:getWareId()
                
                if item:getContainer() then
                    checkContainer(item:getContainer())
                end
                
                if wareId > 0 then
                    itemsWithWareId = itemsWithWareId + 1
                    if #samples < 5 then
                        table.insert(samples, string.format("%s (id:%d, wareId:%d)", itemType:getName(), item:getId(), wareId))
                    end
                else
                    itemsWithoutWareId = itemsWithoutWareId + 1
                end
            end
        end
    end

    checkContainer(depot)

    player:sendTextMessage(MESSAGE_STATUS_CONSOLE_BLUE, string.format("Depot analysis:"))
    player:sendTextMessage(MESSAGE_STATUS_CONSOLE_BLUE, string.format("Items WITH wareId: %d", itemsWithWareId))
    player:sendTextMessage(MESSAGE_STATUS_CONSOLE_BLUE, string.format("Items WITHOUT wareId: %d", itemsWithoutWareId))
    
    if #samples > 0 then
        player:sendTextMessage(MESSAGE_STATUS_CONSOLE_BLUE, "Sample items with wareId:")
        for _, sample in ipairs(samples) do
            player:sendTextMessage(MESSAGE_STATUS_CONSOLE_BLUE, "  " .. sample)
        end
    else
        player:sendTextMessage(MESSAGE_STATUS_CONSOLE_ORANGE, "WARNING: NO items in your depot have wareId set!")
        player:sendTextMessage(MESSAGE_STATUS_CONSOLE_ORANGE, "This means your items.otb file doesn't have market data.")
        player:sendTextMessage(MESSAGE_STATUS_CONSOLE_ORANGE, "You need an items.otb with wareId attributes for the market to work.")
    end

    return false
end

















