--[[
    Hourly Auto-Save
    Saves all players and map data every hour without shutting down the server
]]

local hourlySave = GlobalEvent("HourlyAutoSave")

function hourlySave.onThink(interval)
    -- Save all online players
    local playerCount = 0
    for _, player in ipairs(Game.getPlayers()) do
        player:save()
        playerCount = playerCount + 1
    end
    
    -- Save map (houses, items on ground, etc.)
    saveServer()
    
    -- Log the save
    local timestamp = os.date("%Y-%m-%d %H:%M:%S")
    print(string.format("[%s] Hourly auto-save complete. %d players saved.", timestamp, playerCount))
    
    return true
end

hourlySave:interval(60 * 60 * 1000) -- 1 hour in milliseconds
hourlySave:register()
