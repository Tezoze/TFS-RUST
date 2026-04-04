-- Killing in the Name of... Quest (Paw and Fur Society)
-- Converted from quests.xml to Lua with dynamic task missions
-- Quest NPC: Grizzly Adams (Port Hope)
-- Storage keys defined in data/lib/core/storages.lua under Storage.KillingInTheNameOf

local killingInTheNameOf = GlobalEvent("KillingInTheNameOfQuest")

-- Rank thresholds and names
local rankInfo = {
    {points = 0, name = "Novice"},
    {points = 1, name = "Member"},
    {points = 10, name = "Huntsman"},
    {points = 20, name = "Ranger"},
    {points = 40, name = "Big Game Hunter"},
    {points = 70, name = "Trophy Hunter"},
    {points = 100, name = "Elite Hunter"}
}

local function getRankName(points)
    local rankName = "Novice"
    for _, rank in ipairs(rankInfo) do
        if points >= rank.points then
            rankName = rank.name
        end
    end
    return rankName
end

local function getNextRank(points)
    for _, rank in ipairs(rankInfo) do
        if points < rank.points then
            return rank
        end
    end
    return nil
end

-- Main society description (rank/points overview)
local function getSocietyDescription(player)
    if not player or not player.getStorageValue then return "Player data unavailable." end
    local points = math.max(0, player:getStorageValue(Storage.KillingInTheNameOf.Points) or 0)
    local joinStatus = player:getStorageValue(Storage.KillingInTheNameOf.Join)
    
    if joinStatus < 0 then
        return "Visit Grizzly Adams in Port Hope to join the Paw and Fur - Hunting Elite society!"
    end
    
    local rankName = getRankName(points)
    local nextRank = getNextRank(points)
    
    local desc = "You have joined the Paw and Fur - Hunting Elite society!\n"
    desc = desc .. "Ask Grizzly Adams for a hunting task.\n\n"
    desc = desc .. "Current Rank: " .. rankName .. "\n"
    desc = desc .. "Paw & Fur Points: " .. points .. "\n"
    
    if nextRank then
        local pointsNeeded = nextRank.points - points
        desc = desc .. "\nNext Rank: " .. nextRank.name .. " (" .. pointsNeeded .. " points needed)"
    else
        desc = desc .. "\nYou have achieved the highest rank!"
    end
    
    if points >= 70 then
        desc = desc .. "\n\nSpecial Tasks Available: Tiquanda's Revenge, Demodras"
    end
    
    return desc
end

-- Create description function for a specific task
local function createTaskDescription(taskId)
    return function(player)
        if not player or not player.getStorageValue then return "Player data unavailable." end
        local task = tasks[taskId]
        if not task then return "Task data not found." end
        
        local questStorage = player:getStorageValue(Storage.KillingInTheNameOf.QuestStorageBase + taskId)
        local kills = math.max(0, player:getStorageValue(Storage.KillingInTheNameOf.KillsStorageBase + taskId) or 0)
        local repeats = math.max(0, player:getStorageValue(Storage.KillingInTheNameOf.RepeatStorageBase + taskId) or 0)
        local required = task.killsRequired or 0
        local maxRepeats = task.norepeatable and 1 or repeatTimes
        
        local desc = ""
        
        -- Task is currently active
        if questStorage == 1 then
            if kills >= required then
                desc = "Task Complete! Report to Grizzly Adams to claim your reward.\n\n"
                desc = desc .. "Kills: " .. kills .. "/" .. required .. " (DONE!)\n"
            else
                desc = "Hunt " .. task.raceName .. " and report back to Grizzly Adams.\n\n"
                desc = desc .. "Progress: " .. kills .. "/" .. required .. "\n"
                local remaining = required - kills
                desc = desc .. "Remaining: " .. remaining .. " kills"
            end
        -- Task completed (awaiting new start or maxed out)
        elseif questStorage >= 2 or repeats >= maxRepeats then
            desc = "Task Completed!\n\n"
            if task.norepeatable then
                desc = desc .. "This task cannot be repeated."
            else
                desc = desc .. "Completions: " .. repeats .. "/" .. maxRepeats .. "\n"
                if repeats >= maxRepeats then
                    desc = desc .. "Maximum completions reached."
                else
                    desc = desc .. "You can repeat this task " .. (maxRepeats - repeats) .. " more time(s)."
                end
            end
        -- Task not yet started but we're showing it (shouldn't happen with isStarted check)
        else
            desc = "Talk to Grizzly Adams to start this task."
        end
        
        return desc
    end
end

-- Check if player has interacted with this task
local function createTaskStartedCheck(taskId)
    return function(player)
        if not player or not player.getStorageValue then return false end
        local questStorage = player:getStorageValue(Storage.KillingInTheNameOf.QuestStorageBase + taskId) or 0
        local repeats = player:getStorageValue(Storage.KillingInTheNameOf.RepeatStorageBase + taskId) or 0
        -- Show if task is active (1) or has been completed at least once
        return questStorage >= 1 or repeats >= 1
    end
end

-- Check if task is completed permanently (all repeats done)
local function createTaskCompletedCheck(taskId)
    return function(player)
        if not player or not player.getStorageValue then return false end
        local task = tasks[taskId]
        if not task then return false end
        
        local repeats = math.max(0, player:getStorageValue(Storage.KillingInTheNameOf.RepeatStorageBase + taskId) or 0)
        local maxRepeats = task.norepeatable and 1 or repeatTimes
        return repeats >= maxRepeats
    end
end

function killingInTheNameOf.onStartup()
    -- Wait for tasks table to be loaded
    if not tasks then
        print("[Warning] Killing in the Name of quest: tasks table not loaded yet")
        return false
    end
    
    -- Build missions list: main society + all tasks
    local missionsList = {
        {
            name = "Paw and Fur Society",
            storageId = Storage.KillingInTheNameOf.Join,
            startValue = 0,
            endValue = 999999,
            ignoreEndValue = true,
            description = getSocietyDescription
        }
    }
    
    -- Add each task as its own mission
    for taskId, task in pairs(tasks) do
        local missionName = task.raceName or task.name or ("Task " .. taskId)
        table.insert(missionsList, {
            name = missionName,
            storageId = Storage.KillingInTheNameOf.QuestStorageBase + taskId,
            startValue = 1,
            endValue = 999999,
            ignoreEndValue = true,
            description = createTaskDescription(taskId),
            isStarted = createTaskStartedCheck(taskId),
            isCompleted = createTaskCompletedCheck(taskId)
        })
    end
    
    local quest = Game.createQuest("Killing in the Name of...", {
        storageId = Storage.KillingInTheNameOf.Join,
        storageValue = 0,
        missions = missionsList
    })

    quest:register()
    print(">> Registered quest: Killing in the Name of... (" .. #missionsList .. " missions)")
    return true
end

killingInTheNameOf:register()
