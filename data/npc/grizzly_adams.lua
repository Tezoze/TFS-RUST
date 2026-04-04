-- Grizzly Adams - Converted from XML to Lua NpcType
-- Original XML: data/npc/Grizzly Adams.xml
-- Original Script: data/npc/scripts/Grizzly Adams.lua

local npcName = "Grizzly Adams"
local npcType = Game.createNpcType(npcName)

-- NPC Properties (from XML)
npcType:name(npcName)
npcType:nameDescription("a grizzly adams")
npcType:health(100)
npcType:maxHealth(100)
npcType:walkInterval(2000)
npcType:walkRadius(2)
npcType:baseSpeed(100)
npcType:floorChange(false)
npcType:isPushable(false)
npcType:outfit({lookType = 144, lookHead = 116, lookBody = 78, lookLegs = 94, lookFeet = 78, lookAddons = 3})
npcType:speechBubble(SPEECHBUBBLE_TRADE)

-- Original Lua script content
local keywordHandler = KeywordHandler:new()
local npcHandler = NpcHandler:new(keywordHandler)


-- Boss mapping for task completion notifications (only active bosses)
local bosses = {
    [34100] = {bossName = 'the snapper'},
    [34101] = {bossName = 'hide'},
    -- [34102] = {bossName = 'deathbine'}, -- commented out in boss.lua
    [34103] = {bossName = 'the bloodtusk'},
    [34104] = {bossName = 'shardhead'},
    [34105] = {bossName = 'esmeralda'},
    -- [34106] = {bossName = 'fleshcrawler'}, -- commented out in boss.lua
    -- [34107] = {bossName = 'ribstride'}, -- commented out in boss.lua
    -- [34108] = {bossName = 'the bloodweb'}, -- commented out in boss.lua
    [34109] = {bossName = 'thul'},
    [34110] = {bossName = 'the old widow'},
    [34111] = {bossName = 'hemming'},
    -- [34112] = {bossName = 'tormentor'}, -- commented out in boss.lua
    -- [34113] = {bossName = 'flameborn'}, -- commented out in boss.lua
    -- [34114] = {bossName = 'fazzrah'}, -- commented out in boss.lua
    -- [34115] = {bossName = 'tromphonyte'}, -- commented out in boss.lua
    -- [34116] = {bossName = 'sulphur scuttler'}, -- commented out in boss.lua
    -- [34117] = {bossName = 'bruise payne'}, -- commented out in boss.lua
    [34118] = {bossName = 'the many'},
    [34119] = {bossName = 'the noxious spawn'},
    -- [34120] = {bossName = 'gorgo'}, -- commented out in boss.lua
    [34121] = {bossName = 'stonecracker'},
    [34122] = {bossName = 'leviathan'},
    -- [34123] = {bossName = 'kerberos'}, -- commented out in boss.lua
    [34124] = {bossName = 'ethershreck'},
    -- [34125] = {bossName = 'paiz the pauperizer'}, -- commented out in boss.lua
    -- [34126] = {bossName = 'bretzecutioner'}, -- commented out in boss.lua
    [34127] = {bossName = 'zanakeph'}
}

local choose = {}
local cancel = {}

local grizzlyAdamsConfig = {
    ranks = {
        huntsMan_rank = {
            {id=11214, buy=0, sell=50, name='antlers'},
            {id=10550, buy=0, sell=100, name='bloody pincers'},
            {id=11189, buy=0, sell=35, name='crab pincers'},
            {id=10574, buy=0, sell=55, name='cyclops toe'},
            {id=13303, buy=0, sell=550, name="cavebear skull"},
            {id=12470, buy=0, sell=110, name="colourful feather"},
            {id=7398, buy=0, sell=500, name='cyclops trophy'},
            {id=11315, buy=0, sell=15000, name='draken trophy'},
            {id=13296, buy=0, sell=800, name='draptor scales'},
            {id=21311, buy=0, sell=115, name='elven hoof'},
            {id=10565, buy=0, sell=30, name='frosty ear of a troll'},
            {id=13304, buy=0, sell=950, name='giant crab pincer'},
            {id=12495, buy=0, sell=20, name='goblin ear'},
            {id=13301, buy=0, sell=400, name='hollow stampor hoof'},
            {id=11199, buy=0, sell=600, name='hydra head'},
            {id=11372, buy=0, sell=80, name='lancer beetle shell'},
            {id=11336, buy=0, sell=8000, name='lizard trophy'},
            {id=12445, buy=0, sell=280, name='mantassin tail'},
            {id=19741, buy=0, sell=65, name='marsh stalker beak'},
            {id=19742, buy=0, sell=50, name='marsh stalker feather'},
            {id=13302, buy=0, sell=250, name='maxilla'},
            {id=7401, buy=0, sell=500, name='minotaur trophy'},
            {id=10579, buy=0, sell=420, name='mutated bat ear'},
            {id=13026, buy=0, sell=750, name='panther head'},
            {id=13027, buy=0, sell=300, name='panther paw'},
            {id=12447, buy=0, sell=500, name='quara bone'},
            {id=12447, buy=0, sell=350, name='quara eye'},
            {id=12446, buy=0, sell=410, name='quara pincers'},
            {id=12443, buy=0, sell=140, name='quara tentacle'},
            {id=13159, buy=0, sell=50, name='rabbit\'s foot'},
            {id=21310, buy=0, sell=70, name='rorc feather'},
            {id=11228, buy=0, sell=400, name='sabretooth'},
            {id=11373, buy=0, sell=20, name='sandcrawler shell'},
            {id=10548, buy=0, sell=280, name='scarab pincers'},
            {id=13299, buy=0, sell=280, name='stampor horn'},
            {id=13300, buy=0, sell=150, name='stampor talons'},
            {id=11371, buy=0, sell=60, name='terramite legs'},
            {id=11369, buy=0, sell=170, name='terramite shell'},
            {id=11190, buy=0, sell=95, name='terrorbird beak'},
            -- BUY OFFERS
            {id=2153, buy=10000, sell=0, name='task gem'}
        },
        bigGameHunter_rank = {
            {id=11161, buy=0, sell=6000, name='bonebeast trophy'},
            {id=7397, buy=0, sell=3000, name='deer trophy'},
            {id=7400, buy=0, sell=3000, name='lion trophy'},
            {id=7395, buy=0, sell=1000, name='orc trophy'},
            {id=7394, buy=0, sell=3000, name='wolf trophy'}
        },
        trophyHunter_rank = {
            {id=7396, buy=0, sell=20000, name='behemoth trophy'},
            {id=7393, buy=0, sell=40000, name='demon trophy'},
            {id=7399, buy=0, sell=10000, name='dragon lord trophy'},
            {id=10518, buy=1000, sell=0, name='demon backpack'},
        },
    }
}

-- Pre-process items for shop optimization
local items = {}
for _, rankTable in pairs(grizzlyAdamsConfig.ranks) do
    for _, itemData in ipairs(rankTable) do
        items[itemData.id] = {id = itemData.id, buy = itemData.buy, sell = itemData.sell, name = ItemType(itemData.id):getName():lower()}
    end
end

local function greetCallback(cid)
    local player = Player(cid)
    if player:getStorageValue(Storage.KillingInTheNameOf.Join) == -1 then
        npcHandler:setMessage(MESSAGE_GREET, 'Welcome |PLAYERNAME|. Would you like to join the \'Paw and Fur - Hunting Elite\'?')
    else
        npcHandler:setMessage(MESSAGE_GREET, 'Welcome back old chap. What brings you here this time?')
    end
    return true
end

local function joinTables(old, new)
    for k, v in pairs(new) do old[#old+1] = v end
    return old
end

local function onBuy(cid, item, subType, amount, ignoreCap, inBackpacks)
    local player = Player(cid)
    local itemData = items[item]
    if not itemData then return false end

    if not ignoreCap and player:getFreeCapacity() < ItemType(itemData.id):getWeight(amount) then
        return player:sendTextMessage(MESSAGE_INFO_DESCR, 'You don\'t have enough cap.')
    end
    if not player:removeMoney(itemData.buy * amount) then
        selfSay("You don't have enough money.", cid)
    else
        for i = 1, amount do
            local purchasedItem = player:addItem(itemData.id, 1)
            if itemData.id == 2153 and purchasedItem then
                purchasedItem:setActionId(25000)
            end
        end
        return player:sendTextMessage(MESSAGE_INFO_DESCR, 'Bought '..amount..'x '..itemData.name..' for '..itemData.buy * amount..' gold coins.')
    end
    return true
end

local function onSell(cid, item, subType, amount, ignoreCap, inBackpacks)
    local player = Player(cid)
    local itemData = items[item]
    if not itemData then return false end

    if player:removeItem(itemData.id, amount) then
        player:addMoney(itemData.sell * amount)
        return player:sendTextMessage(MESSAGE_INFO_DESCR, 'Sold '..amount..'x '..itemData.name..' for '..itemData.sell * amount..' gold coins.')
    else
        selfSay("You don't have item to sell.", cid)
    end
    return true
end

local function creatureSayCallback(cid, msgType, msg)
    if not npcHandler:isFocused(cid) then return false end
    local player = Player(cid)
    msg = msg:gsub("(%l)(%w*)", function(a,b) return string.upper(a)..b end)

    if msgcontains('trade', msg) then
        local tradeItems = {}
        if player:getPawAndFurRank() >= 1 then
            tradeItems = grizzlyAdamsConfig.ranks.huntsMan_rank
            if player:getPawAndFurRank() == 4 then
                tradeItems = joinTables(tradeItems, grizzlyAdamsConfig.ranks.bigGameHunter_rank)
            elseif player:getPawAndFurRank() >= 5 then
                tradeItems = joinTables(tradeItems, grizzlyAdamsConfig.ranks.bigGameHunter_rank)
                tradeItems = joinTables(tradeItems, grizzlyAdamsConfig.ranks.trophyHunter_rank)
            end
            openShopWindow(cid, tradeItems, onBuy, onSell)
            return npcHandler:say('It\'s my offer.', cid)
        else
            return npcHandler:say('You\'ll have to {join}, to get access to my shop.', cid)
        end

    elseif (msgcontains('join', msg) or msgcontains('yes', msg)) and npcHandler.topic[cid] == 0 
    and (player:getStorageValue(Storage.KillingInTheNameOf.Join) == -1) then
        player:setStorageValue(Storage.KillingInTheNameOf.Join, 0)
        player:setStorageValue(2501, 0)
        
        player:registerEvent("KillingInTheNameOfKills")
        player:registerEvent("KillingInTheNameOfKillss")
        player:registerEvent("KillingInTheNameOfKillsss")
        player:registerEvent("KillingInTheNameOfLogout")
        npcHandler:say('Great!, now you can start tasks.', cid)

    elseif isInArray({'tasks', 'task', 'mission'}, msg:lower()) then
        if player:getStorageValue(Storage.KillingInTheNameOf.Join) == -1 then
            return npcHandler:say('You\'ll have to {join}, to get any {tasks}.',cid)
        end

        local currentPoints = player:getPawAndFurPoints()
        local eligibleForPromotion = false

        if (currentPoints >= 10 and player:getStorageValue(Storage.KillingInTheNameOf.PromotionHuntsman) < 1) or
           (currentPoints >= 20 and player:getStorageValue(Storage.KillingInTheNameOf.PromotionRanger) < 1) or
           (currentPoints >= 40 and player:getStorageValue(Storage.KillingInTheNameOf.PromotionBigGameHunter) < 1) or
           (currentPoints >= 70 and player:getStorageValue(Storage.KillingInTheNameOf.PromotionTrophyHunter) < 1) or
           (currentPoints >= 100 and player:getStorageValue(Storage.KillingInTheNameOf.PromotionEliteHunter) < 1) then
            eligibleForPromotion = true
        end

        if eligibleForPromotion then
            npcHandler:say('You are ready to advance one rank in our society ' .. player:getName() .. '. Ask me for a {promotion} first.', cid)
            npcHandler.topic[cid] = 0
            return true
        end

        local can = player:getTasks()
        if #can > 0 then
            local taskNames = {}
            for i = 1, #can do
                table.insert(taskNames, '{' .. (tasks[can[i]].name or tasks[can[i]].raceName) .. '}')
            end
            npcHandler:say('The current tasks that you can choose are: ' .. table.concat(taskNames, ", ") .. '.', cid)
            npcHandler.topic[cid] = 0
        else
            npcHandler:say('I don\'t have any task for you right now.', cid)
        end

    elseif msg ~= '' and player:canStartTask(msg) then
        if #player:getStartedTasks() >= tasksByPlayer then
            npcHandler:say('Sorry, but you already started ' .. tasksByPlayer .. ' tasks. You can check their {status}, {cancel} or {report} a task.', cid)
            return true
        end
        local task = getTaskByName(msg)
        if task then
            local questStorage = player:getStorageValue(Storage.KillingInTheNameOf.QuestStorageBase + task)
            if questStorage and questStorage > 0 then return false end
            
            npcHandler:say('In this task you must defeat ' .. tasks[task].killsRequired .. ' ' .. tasks[task].raceName .. '. Are you sure that you want to start this task?', cid)
            choose[cid] = task
            npcHandler.topic[cid] = 1
        end

    elseif msg:lower() == 'yes' and npcHandler.topic[cid] == 1 then
        player:setStorageValue(Storage.KillingInTheNameOf.QuestStorageBase + choose[cid], 1)
        player:setStorageValue(Storage.KillingInTheNameOf.KillsStorageBase + choose[cid], 0)
        player:setStorageValue(2501, #player:getStartedTasks())
        
        player:registerEvent("KillingInTheNameOfKills")
        
        npcHandler:say('Excellent! You can check the {status} of your task saying {report} to me. Also you can {cancel} tasks to.', cid)
        choose[cid] = nil
        npcHandler.topic[cid] = 0

    elseif msgcontains('status', msg) then
        local started = player:getStartedTasks()
        if started and #started > 0 then
            local text = ''
            for i = 1, #started do
                local id = started[i]
                local kills = math.max(0, player:getStorageValue(Storage.KillingInTheNameOf.KillsStorageBase + id))
                text = text .. 'Task: ' .. tasks[id].raceName .. ' (' .. kills .. '/' .. tasks[id].killsRequired .. ').'
                
                -- FIX: No Newline on the last entry
                if i < #started then
                    text = text .. '\n'
                end
            end
            npcHandler:say('The status of your current tasks is:\n' .. text, cid)
        else
            npcHandler:say('You haven\'t started any task yet.', cid)
        end

    elseif msgcontains('report', msg) then
        local started = player:getStartedTasks()
        local finishedAtLeastOne = false
        local finishedCount = 0
        local bossesUnlocked = {} -- Store unlocked bosses here

        -- 1. Check Special Tasks (Demodras/Tiquanda)
        local specialTasks = {}
        if player:getStorageValue(Storage.KillingInTheNameOf.MissionTiquandasRevenge) == 1 then
            if player:getStorageValue(Storage.KillingInTheNameOf.TiquandasRevengeTeleport) == 0 then
                table.insert(specialTasks, {name = "Tiquandas Revenge", status = "completed"})
                finishedAtLeastOne = true
                finishedCount = finishedCount + 1
            else
                table.insert(specialTasks, {name = "Tiquandas Revenge", status = "in progress"})
            end
        end
        if player:getStorageValue(Storage.KillingInTheNameOf.MissionDemodras) >= 1 then
            if player:getStorageValue(Storage.KillingInTheNameOf.MissionDemodras) == 2 then
                table.insert(specialTasks, {name = "Demodras", status = "completed"})
                finishedAtLeastOne = true
                finishedCount = finishedCount + 1
            else
                table.insert(specialTasks, {name = "Demodras", status = "in progress"})
            end
        end

        -- 2. Check Standard Tasks
        if started and #started > 0 then
            for i = 1, #started do
                local id = started[i]
                local killsStorage = player:getStorageValue(Storage.KillingInTheNameOf.KillsStorageBase + id)
                if killsStorage and killsStorage >= tasks[id].killsRequired then
                    -- Reward Loop
                    for j = 1, #tasks[id].rewards do
                        local reward = tasks[id].rewards[j]
                        local deny = false
                        if reward.storage then
                            local rewardStorage = math.max(0, player:getStorageValue(reward.storage[1]))
                            if rewardStorage >= reward.storage[2] then deny = true end
                        end
                        
                        if not deny then
                            if isInArray({REWARD_MONEY, 'money'}, reward.type:lower()) then player:addMoney(reward.value[1])
                            elseif isInArray({REWARD_EXP, 'exp', 'experience'}, reward.type:lower()) then 
                                -- Apply experience stage multiplier based on player level
                                -- Stages mirror config.lua experienceStages
                                local baseExp = reward.value[1]
                                local multiplier = 1
                                local playerLevel = player:getLevel()
                                
                                local stages = {
                                    	{ minlevel = 1, maxlevel = 20, multiplier = 5 },  
	                                    { minlevel = 21, maxlevel = 40, multiplier = 4 },  
	                                    { minlevel = 41, maxlevel = 60, multiplier = 3 },   
	                                    { minlevel = 61, maxlevel = 80, multiplier = 2.5 },   
	                                    { minlevel = 81, maxlevel = 120, multiplier = 2 },  
	                                    { minlevel = 121, maxlevel = 200, multiplier = 1.5 }, 
	                                    { minlevel = 201, multiplier = 1 }  
                                }
                                
                                for _, stage in ipairs(stages) do
                                    if playerLevel >= stage.minlevel and (not stage.maxlevel or playerLevel <= stage.maxlevel) then
                                        multiplier = stage.multiplier
                                        break
                                    end
                                end
                                
                                local finalExp = baseExp * multiplier
                                player:addExperience(finalExp, true)
                            elseif isInArray({REWARD_ACHIEVEMENT, 'achievement', 'ach'}, reward.type:lower()) then player:addAchievement(reward.value[1])
                            elseif isInArray({REWARD_POINT, 'points', 'point'}, reward.type:lower()) then 
                                player:setStorageValue(Storage.KillingInTheNameOf.Points, getPlayerTasksPoints(cid) + reward.value[1])
                            elseif isInArray({REWARD_ITEM, 'item', 'items', 'object'}, reward.type:lower()) then player:addItem(reward.value[1], reward.value[2])
                            elseif isInArray({REWARD_STORAGE, 'storage', 'stor'}, reward.type:lower()) then
                                player:setStorageValue(reward.value[1], reward.value[2])
                                -- FIX: Capture Boss Name Here
                                if bosses[reward.value[1]] then
                                    table.insert(bossesUnlocked, bosses[reward.value[1]].bossName)
                                end
                            end
                        end
                        if reward.storage then
                            player:setStorageValue(reward.storage[1], reward.storage[2])
                        end
                    end

                    -- Mark Completed
                    player:setStorageValue(Storage.KillingInTheNameOf.QuestStorageBase + id, (tasks[id].norepeatable and 2 or 0))
                    player:setStorageValue(Storage.KillingInTheNameOf.KillsStorageBase + id, -1)
                    local repeats = math.max(0, player:getStorageValue(Storage.KillingInTheNameOf.RepeatStorageBase + id))
                    player:setStorageValue(Storage.KillingInTheNameOf.RepeatStorageBase + id, repeats + 1)
                    
                    finishedAtLeastOne = true
                    finishedCount = finishedCount + 1
                end
            end
        end

        if finishedAtLeastOne then
            player:setStorageValue(2501, #player:getStartedTasks())
            local msg = 'Awesome! you finished ' .. finishedCount .. ' task' .. (finishedCount > 1 and 's' or '') .. '.'
            
            if #bossesUnlocked > 0 then
                msg = msg .. ' Your efforts have awakened ' .. table.concat(bossesUnlocked, ", ") .. '! You can now challenge these mighty foes.'
            else
                msg = msg .. ' Talk to me again if you want to start a new {task}.'
            end
            npcHandler:say(msg, cid)
        else
            -- No finished tasks, report status
            if #specialTasks > 0 then
                local statusTexts = {}
                for k, v in ipairs(specialTasks) do table.insert(statusTexts, v.name .. " (" .. v.status .. ")") end
                npcHandler:say('Status: ' .. table.concat(statusTexts, ", ") .. '.', cid)
            elseif started and #started > 0 then
                local taskNames = {}
                for i = 1, #started do table.insert(taskNames, '{' .. tasks[started[i]].raceName .. '}') end
                npcHandler:say('The current tasks that you started are ' .. table.concat(taskNames, ", ") .. '.', cid)
            else
                npcHandler:say('You haven\'t started any task yet.', cid)
            end
        end

    elseif msg:lower() == 'started' then
        local started = player:getStartedTasks()
        if started and #started > 0 then
            local taskNames = {}
            for i = 1, #started do table.insert(taskNames, '{' .. tasks[started[i]].raceName .. '}') end
            npcHandler:say('The current tasks that you started are ' .. table.concat(taskNames, ", ") .. '.', cid)
        else
            npcHandler:say('You haven\'t started any task yet.', cid)
        end

    elseif msg:lower() == 'cancel' then
        local started = player:getStartedTasks()
        if started and #started > 0 then
            local taskNames = {}
            for i = 1, #started do table.insert(taskNames, '{' .. tasks[started[i]].raceName .. '}') end
            npcHandler:say('Cancelling a task will make the counter restart. Which of these tasks you want cancel? ' .. table.concat(taskNames, ", "), cid)
            npcHandler.topic[cid] = 2
        else
            npcHandler:say('You haven\'t started any task yet.', cid)
        end

    elseif getTaskByName(msg) and npcHandler.topic[cid] == 2 then
        local task = getTaskByName(msg)
        if isInArray(player:getStartedTasks(), task) then
            npcHandler:say('Are you sure you want to cancel the task ' .. tasks[task].raceName .. '?', cid)
            npcHandler.topic[cid] = 3
            cancel[cid] = task
        else
            npcHandler:say('You have not started that task.', cid)
        end

    elseif msg:lower() == 'yes' and npcHandler.topic[cid] == 3 then
        local taskId = cancel[cid]
        player:setStorageValue(Storage.KillingInTheNameOf.QuestStorageBase + taskId, -1)
        player:setStorageValue(Storage.KillingInTheNameOf.KillsStorageBase + taskId, -1)
        player:setStorageValue(2501, #player:getStartedTasks())
        
        npcHandler:say('You have cancelled the task ' .. tasks[taskId].raceName .. '.', cid)
        npcHandler.topic[cid] = 0

    elseif isInArray({'points', 'rank'}, msg:lower()) then
        local points = player:getPawAndFurPoints()
        local rankNames = {
            [1] = "Member", [2] = "Huntsman", [3] = "Ranger", [4] = "Big Game Hunter", [5] = "Trophy Hunter", [6] = "Elite Hunter"
        }
        local rankName = rankNames[player:getPawAndFurRank()] or "Novice"
        npcHandler:say('You have ' .. points .. ' Paw & Fur points. You are a ' .. rankName .. '.', cid)
        npcHandler.topic[cid] = 0

    elseif msgcontains('promotion', msg) then
        local points = player:getPawAndFurPoints()
        local ranks = {
            {points=10, name='Huntsman', storage=Storage.KillingInTheNameOf.PromotionHuntsman},
            {points=20, name='Ranger', storage=Storage.KillingInTheNameOf.PromotionRanger},
            {points=40, name='Big Game Hunter', storage=Storage.KillingInTheNameOf.PromotionBigGameHunter},
            {points=70, name='Trophy Hunter', storage=Storage.KillingInTheNameOf.PromotionTrophyHunter, reward=10518},
            {points=100, name='Elite Hunter', storage=Storage.KillingInTheNameOf.PromotionEliteHunter}
        }
        
        local promoted = false
        for _, rank in ipairs(ranks) do
            if points >= rank.points and player:getStorageValue(rank.storage) < 1 then
                player:setStorageValue(rank.storage, 1)
                if rank.reward then player:addItem(rank.reward, 1) end
                npcHandler:say('Congratulations! You have reached '..rank.points..' points and are now a '..rank.name..'.', cid)
                promoted = true
                break
            end
        end
        
        if not promoted then
            if player:getStorageValue(Storage.KillingInTheNameOf.PromotionEliteHunter) >= 1 then
                npcHandler:say('You have already reached the highest rank - Elite Hunter.', cid)
            else
                npcHandler:say('You don\'t have enough points for a new promotion yet.', cid)
            end
        end
        npcHandler.topic[cid] = 0

    elseif isInArray({'special task', 'special tasks'}, msg:lower()) then
        if player:getPawAndFurPoints() >= 70 then
            local availableTasks = {}
            local failMsg = ""

            -- Tiquandas Revenge
            if player:getLevel() >= 90 then
                local tStatus = player:getStorageValue(Storage.KillingInTheNameOf.TiquandasRevengeTeleport)
                if tStatus == 2 then failMsg = failMsg .. " You failed Tiquandas Revenge."
                elseif player:getStorageValue(Storage.KillingInTheNameOf.MissionTiquandasRevenge) ~= 1 then table.insert(availableTasks, 'Tiquandas Revenge') end
            end

            -- Demodras
            if player:getLevel() >= 100 then
                local dStatus = player:getStorageValue(Storage.KillingInTheNameOf.DemodrasTeleport)
                if dStatus == 2 then failMsg = failMsg .. " You failed Demodras."
                elseif player:getStorageValue(Storage.KillingInTheNameOf.MissionDemodras) ~= 1 then table.insert(availableTasks, 'Demodras') end
            end

            if #availableTasks > 0 then
                npcHandler:say('Available special tasks: ' .. table.concat(availableTasks, ' and ') .. '.' .. failMsg, cid)
            else
                npcHandler:say('You have no special tasks available.' .. failMsg, cid)
            end
        else
            npcHandler:say('You need to reach Trophy Hunter rank (70 points) to access special tasks.', cid)
        end
        npcHandler.topic[cid] = 0

    elseif isInArray({'tiquandas', 'tiquandas revenge'}, msg:lower()) then
        if player:getPawAndFurPoints() >= 70 and player:getLevel() >= 90 then
            local status = player:getStorageValue(Storage.KillingInTheNameOf.TiquandasRevengeTeleport)
            if status == 2 then
                npcHandler:say('You have failed this task. You cannot repeat it.', cid)
            elseif player:getStorageValue(Storage.KillingInTheNameOf.MissionTiquandasRevenge) == 1 then
                npcHandler:say('You are already on this mission. Go find the hideout!', cid)
            else
                npcHandler:say('Go find the hideout! (Task Started)', cid)
                player:setStorageValue(Storage.KillingInTheNameOf.TiquandasRevengeTeleport, 1)
                player:setStorageValue(Storage.KillingInTheNameOf.MissionTiquandasRevenge, 1)
            end
        else
            npcHandler:say('You do not meet the requirements.', cid)
        end

    elseif isInArray({'demodras', 'demodra'}, msg:lower()) then
        if player:getPawAndFurPoints() >= 70 and player:getLevel() >= 100 then
            local status = player:getStorageValue(Storage.KillingInTheNameOf.DemodrasTeleport)
            if status == 2 then
                npcHandler:say('You have failed this task. You cannot repeat it.', cid)
            elseif player:getStorageValue(Storage.KillingInTheNameOf.MissionDemodras) == 1 then
                npcHandler:say('You are already on this mission. Find Demodras!', cid)
            else
                npcHandler:say('Find Demodras! (Task Started)', cid)
                player:setStorageValue(Storage.KillingInTheNameOf.DemodrasTeleport, 1)
                player:setStorageValue(Storage.KillingInTheNameOf.MissionDemodras, 1)
            end
        else
            npcHandler:say('You do not meet the requirements.', cid)
        end
    end
end

npcHandler:setMessage(MESSAGE_FAREWELL, 'Happy hunting, old chap!')
npcHandler:setCallback(CALLBACK_GREET, greetCallback)
npcHandler:setCallback(CALLBACK_MESSAGE_DEFAULT, creatureSayCallback)


-- NpcType callbacks (MUST call setCurrentNpc first!)
npcType:eventType(NPCS_EVENT_APPEAR)
npcType:onAppear(function(npc, creature)
    setCurrentNpc(npc)
    npcHandler:onCreatureAppear(creature)
end)

npcType:eventType(NPCS_EVENT_DISAPPEAR)
npcType:onDisappear(function(npc, creature)
    setCurrentNpc(npc)
    npcHandler:onCreatureDisappear(creature)
end)

npcType:eventType(NPCS_EVENT_SAY)
npcType:onSay(function(npc, creature, type, message)
    setCurrentNpc(npc)
    npcHandler:onCreatureSay(creature, type, message)
end)

npcType:eventType(NPCS_EVENT_THINK)
npcType:onThink(function(npc, interval)
    setCurrentNpc(npc)
    npcHandler:onThink()
end)

npcType:eventType(NPCS_EVENT_CLOSECHANNEL)
npcType:onCloseChannel(function(npc, creature)
    setCurrentNpc(npc)
    npcHandler:onPlayerCloseChannel(creature)
end)

npcHandler:addModule(FocusModule:new())
npcType:register()
