--[[
    Dungeon System — Loot Distribution
    Personal loot mode: each player rolls independently.
]]

DungeonLoot = {}

function DungeonLoot.distributeLoot(key, encounter)
    local instance = DungeonManager.activeInstances[key]
    if not instance then return end

    local droppedItems = {}

    -- Roll loot
    for _, lootEntry in ipairs(encounter.loot or {}) do
        if math.random(1, 100000) <= lootEntry.chance then
            local count = 1
            if lootEntry.count then
                count = math.random(lootEntry.count[1], lootEntry.count[2])
            end
            table.insert(droppedItems, {
                itemId = lootEntry.itemId,
                count = count,
                name = lootEntry.name,
            })
        end
    end

    if #droppedItems == 0 then
        DungeonManager.broadcastToInstance(key, {
            action = "loot_dropped",
            items = {},
            message = "No loot dropped this time.",
        })
        return
    end

    -- Personal loot: give to each connected player
    for _, pd in ipairs(instance.players) do
        local player = Player(pd.id)
        if player and pd.connected then
            for _, item in ipairs(droppedItems) do
                player:addItem(item.itemId, item.count)
            end
        end
    end

    -- Notify
    DungeonManager.broadcastToInstance(key, {
        action = "loot_dropped",
        items = droppedItems,
    })
end
