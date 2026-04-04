--[[
    Modern Task System (Server-Side) Revamped
    Features: Daily/Weekly Random Pools, Rarity Rewards, Seeded Selection.
    Protocol: 205
    
    Storage Range: 90000-90699 (defined in data/lib/core/storages.lua)
]]

local OPCODE = 205

local config = {
    -- Storage (uses Storage.TaskSystem from storages.lua)
    storage = {
        points = Storage.TaskSystem.Points,
        daily_seed = Storage.TaskSystem.DailySeed,
        daily_count = Storage.TaskSystem.DailyCount,
        weekly_seed = Storage.TaskSystem.WeeklySeed,
        weekly_count = Storage.TaskSystem.WeeklyCount,
        kills_start = Storage.TaskSystem.KillsStorageBase,
        state_start = Storage.TaskSystem.StateStorageBase,
        reward_preference = Storage.TaskSystem.RewardPreference,
    },
    
    limits = {
        daily = 5,
        weekly = 3
    },
    
    -- Extend task configuration
    extend = {
        cost = 30,              -- Points cost to extend a task
        maxProgress = 50,       -- Can only extend if less than 50% complete
        multiplier = 2          -- Doubles kill count and rewards
    },
    
    -- Gold rewards are now defined per-task (type="gold") instead of converting from EXP.
    -- This ratio is kept for backward compatibility with the client UI display.
    -- It is NO LONGER used in the claim logic.
    expToGoldRatio = 2,

    shop = {
        -- Potions (100x each) - 1 point = 1100 gold
        -- Format: serverId for addItem, clientId for UI display
        {name = "100x Health Potion", desc = "Basic healing potion", cost = 4, clientId = 266, serverId = 7618, category = "Potions", onBuy = function(p) p:addItem(7618, 100) p:sendTextMessage(MESSAGE_EVENT_ADVANCE, "You received 100 Health Potions!") end},
        {name = "100x Mana Potion", desc = "Basic mana potion", cost = 5, clientId = 268, serverId = 7620, category = "Potions", onBuy = function(p) p:addItem(7620, 100) p:sendTextMessage(MESSAGE_EVENT_ADVANCE, "You received 100 Mana Potions!") end},
        {name = "100x Strong Health Potion", desc = "Strong healing potion", cost = 9, clientId = 236, serverId = 7588, category = "Potions", onBuy = function(p) p:addItem(7588, 100) p:sendTextMessage(MESSAGE_EVENT_ADVANCE, "You received 100 Strong Health Potions!") end},
        {name = "100x Strong Mana Potion", desc = "Strong mana potion", cost = 7, clientId = 237, serverId = 7589, category = "Potions", onBuy = function(p) p:addItem(7589, 100) p:sendTextMessage(MESSAGE_EVENT_ADVANCE, "You received 100 Strong Mana Potions!") end},
        {name = "100x Great Health Potion", desc = "Great healing potion", cost = 17, clientId = 239, serverId = 7591, category = "Potions", onBuy = function(p) p:addItem(7591, 100) p:sendTextMessage(MESSAGE_EVENT_ADVANCE, "You received 100 Great Health Potions!") end},
        {name = "100x Great Mana Potion", desc = "Great mana potion", cost = 11, clientId = 238, serverId = 7590, category = "Potions", onBuy = function(p) p:addItem(7590, 100) p:sendTextMessage(MESSAGE_EVENT_ADVANCE, "You received 100 Great Mana Potions!") end},
        {name = "100x Great Spirit Potion", desc = "Great spirit potion", cost = 17, clientId = 7642, serverId = 8472, category = "Potions", onBuy = function(p) p:addItem(8472, 100) p:sendTextMessage(MESSAGE_EVENT_ADVANCE, "You received 100 Great Spirit Potions!") end},
        {name = "100x Ultimate Health Potion", desc = "Ultimate healing potion", cost = 28, clientId = 7643, serverId = 8473, category = "Potions", onBuy = function(p) p:addItem(8473, 100) p:sendTextMessage(MESSAGE_EVENT_ADVANCE, "You received 100 Ultimate Health Potions!") end},
        
        -- Runes (100x each)
        {name = "100x Sudden Death Rune", desc = "Deadly magic rune", cost = 15, clientId = 3155, serverId = 2268, category = "Runes", onBuy = function(p) p:addItem(2268, 100) p:sendTextMessage(MESSAGE_EVENT_ADVANCE, "You received 100 Sudden Death Runes!") end},
        {name = "100x Ultimate Healing Rune", desc = "Powerful healing rune", cost = 16, clientId = 3160, serverId = 2273, category = "Runes", onBuy = function(p) p:addItem(2273, 100) p:sendTextMessage(MESSAGE_EVENT_ADVANCE, "You received 100 Ultimate Healing Runes!") end},
        {name = "100x Great Fireball Rune", desc = "Area fire damage", cost = 6, clientId = 3191, serverId = 2304, category = "Runes", onBuy = function(p) p:addItem(2304, 100) p:sendTextMessage(MESSAGE_EVENT_ADVANCE, "You received 100 Great Fireball Runes!") end},
        {name = "100x Avalanche Rune", desc = "Area ice damage", cost = 5, clientId = 3161, serverId = 2274, category = "Runes", onBuy = function(p) p:addItem(2274, 100) p:sendTextMessage(MESSAGE_EVENT_ADVANCE, "You received 100 Avalanche Runes!") end},
        {name = "100x Thunderstorm Rune", desc = "Area energy damage", cost = 4, clientId = 3202, serverId = 2315, category = "Runes", onBuy = function(p) p:addItem(2315, 100) p:sendTextMessage(MESSAGE_EVENT_ADVANCE, "You received 100 Thunderstorm Runes!") end},
        {name = "100x Stone Shower Rune", desc = "Area earth damage", cost = 4, clientId = 3175, serverId = 2288, category = "Runes", onBuy = function(p) p:addItem(2288, 100) p:sendTextMessage(MESSAGE_EVENT_ADVANCE, "You received 100 Stone Shower Runes!") end},
        
        -- Ammunition (150x for Basic, 100x for Advanced)
        {name = "150x Onyx Arrow", desc = "Dark arrows", cost = 1, clientId = 7365, serverId = 7365, category = "Ammunition", onBuy = function(p) p:addItem(7365, 150) p:sendTextMessage(MESSAGE_EVENT_ADVANCE, "You received 150 Onyx Arrows!") end},
        {name = "150x Power Bolt", desc = "Strong bolts", cost = 1, clientId = 3450, serverId = 2547, category = "Ammunition", onBuy = function(p) p:addItem(2547, 150) p:sendTextMessage(MESSAGE_EVENT_ADVANCE, "You received 150 Power Bolts!") end},
        {name = "100x Burst Arrow", desc = "Explosive arrows", cost = 1, clientId = 3449, serverId = 2546, category = "Ammunition", onBuy = function(p) p:addItem(2546, 100) p:sendTextMessage(MESSAGE_EVENT_ADVANCE, "You received 100 Burst Arrows!") end},
        {name = "100x Crystalline Arrow", desc = "Powerful arrows", cost = 2, clientId = 15793, serverId = 18304, category = "Ammunition", onBuy = function(p) p:addItem(18304, 100) p:sendTextMessage(MESSAGE_EVENT_ADVANCE, "You received 100 Crystalline Arrows!") end},
        {name = "100x Prismatic Bolt", desc = "Powerful bolts", cost = 2, clientId = 16141, serverId = 18435, category = "Ammunition", onBuy = function(p) p:addItem(18435, 100) p:sendTextMessage(MESSAGE_EVENT_ADVANCE, "You received 100 Prismatic Bolts!") end},
        
        -- Upgrades (Rarity Gems)
        {name = "Rare Gem", desc = "Upgrade Normal to Rare. It can fail.", cost = 30, clientId = 16125, serverId = 18419, category = "Upgrades", onBuy = function(p) p:addItem(18419, 1) end},
        {name = "Epic Gem", desc = "Upgrade Rare to Epic. It can fail and go backwards.", cost = 60, clientId = 16120, serverId = 18414, category = "Upgrades", onBuy = function(p) p:addItem(18414, 1) end},
        {name = "Legendary Coin", desc = "Upgrade Epic to Legendary. It can fail and go backwards.", cost = 150, clientId = 21746, serverId = 24115, category = "Upgrades", onBuy = function(p) p:addItem(24115, 1) end},
        {name = "Mythic Gem", desc = "Upgrade Legendary to Mythic. It can fail and go backwards.", cost = 300, clientId = 16126, serverId = 18420, category = "Upgrades", onBuy = function(p) p:addItem(18420, 1) end},
        {name = "Reroll Scroll", desc = "Reroll attributes, keep rarity.", cost = 120, clientId = 16119, serverId = 18413, category = "Upgrades", onBuy = function(p) p:addItem(18413, 1) end},
        
        -- Bonus Items
        -- {name = "Experience Boost", desc = "+50% exp for 1 hour", cost = 10, clientId = 3031, serverId = 3031, category = "Upgrades", onBuy = function(p) local c = Condition(CONDITION_ATTRIBUTES) c:setParameter(CONDITION_PARAM_TICKS, 3600000) c:setParameter(CONDITION_PARAM_SKILL_EXPERIENCEPERCENT, 150) p:addCondition(c) p:sendTextMessage(MESSAGE_EVENT_ADVANCE, "Exp boost active!") end},
        -- {name = "Stamina Refill", desc = "Restore 1 hour stamina", cost = 13, clientId = 3031, serverId = 3035, category = "Upgrades", onBuy = function(p) p:setStamina(math.min(p:getStamina() + 60, 2520)) p:sendTextMessage(MESSAGE_EVENT_ADVANCE, "Stamina restored!") end},
        -- {name = "Blessing Bundle", desc = "All 5 blessings", cost = 22, clientId = 3031, serverId = 3043, category = "Upgrades", onBuy = function(p) for i=1,5 do p:addBlessing(i) end p:sendTextMessage(MESSAGE_EVENT_ADVANCE, "Blessed!") end}
        
        -- Special Upgrades (permanent - have storageKey for purchased tracking)
        {name = "Slayer Essence", desc = "+20% Dmg vs Task Monsters (Perm)", cost = 400, clientId = 2353, serverId = 2353, category = "Upgrades",
            storageKey = "SlayerEssence",
            onBuy = function(p) 
                if p:getStorageValue(Storage.TaskSystem.SlayerEssence) == 1 then
                    p:sendTextMessage(MESSAGE_STATUS_SMALL, "You already have this upgrade!")
                    return false -- Refund handled in logic
                end
                p:setStorageValue(Storage.TaskSystem.SlayerEssence, 1) 
                p:sendTextMessage(MESSAGE_EVENT_ADVANCE, "You have unlocked Slayer Essence! +20% Damage vs Task Monsters.") 
                return true
            end
        },
        {name = "Bigger and Badder", desc = "Chance to spawn Elite monsters (Perm)", cost = 350, clientId = 3114, serverId = 2229, category = "Upgrades",
            storageKey = "BiggerAndBadder",
            onBuy = function(p) 
                if p:getStorageValue(Storage.TaskSystem.BiggerAndBadder) == 1 then
                    p:sendTextMessage(MESSAGE_STATUS_SMALL, "You already have this upgrade!")
                    return false
                end
                p:setStorageValue(Storage.TaskSystem.BiggerAndBadder, 1) 
                p:sendTextMessage(MESSAGE_EVENT_ADVANCE, "You have unlocked Bigger and Badder! Task monsters may now spawn Elite versions.") 
                return true
            end
        }
    },

    pool = {
        daily = {
            -- Daily Tasks - 5 clean level brackets, no overlap
            -- Points scale UP with level so high-level content is always the best way to earn
            
            -- BRACKET 1: Level 8-30 (2-4 pts, 120-180 kills, 5.4k-10.8k gold)
            {name = "Troll Trouble", mobs = {"troll", "swamp troll"}, count = 120, rewards = {{type="exp", value=3600}, {type="gold", value=5400}, {type="points", value=2}}, minLevel = 8, maxLevel = 30},
            {name = "Minotaur Maze", mobs = {"minotaur", "minotaur archer", "minotaur guard", "minotaur mage"}, count = 165, rewards = {{type="exp", value=5400}, {type="gold", value=8100}, {type="points", value=2}}, minLevel = 8, maxLevel = 30},
            {name = "Orc Patrol", mobs = {"orc", "orc spearman", "orc warrior", "orc berserker"}, count = 165, rewards = {{type="exp", value=5400}, {type="gold", value=8100}, {type="points", value=2}}, minLevel = 8, maxLevel = 30},
            {name = "Rotworm Cleanup", mobs = {"rotworm", "carrion worm"}, count = 150, rewards = {{type="exp", value=3600}, {type="gold", value=5400}, {type="points", value=2}}, minLevel = 8, maxLevel = 30},
            {name = "Larva Feast", mobs = {"larva"}, count = 180, rewards = {{type="exp", value=2700}, {type="gold", value=5400}, {type="points", value=2}}, minLevel = 8, maxLevel = 30},
            {name = "Crocodile Hunter", mobs = {"crocodile"}, count = 135, rewards = {{type="exp", value=4500}, {type="gold", value=6480}, {type="points", value=2}}, minLevel = 8, maxLevel = 30},
            {name = "Elf Scouts", mobs = {"elf", "elf scout", "elf arcanist"}, count = 135, rewards = {{type="exp", value=4500}, {type="gold", value=6480}, {type="points", value=2}}, minLevel = 8, maxLevel = 30},
            {name = "Goblin Hunt", mobs = {"goblin", "goblin assassin"}, count = 135, rewards = {{type="exp", value=2700}, {type="gold", value=5400}, {type="points", value=2}}, minLevel = 8, maxLevel = 30},
            {name = "Dworc Village", mobs = {"dworc fleshhunter", "dworc venomsniper", "dworc voodoomaster"}, count = 150, rewards = {{type="exp", value=3600}, {type="gold", value=6480}, {type="points", value=2}}, minLevel = 8, maxLevel = 30},
            {name = "Cyclops Camp", mobs = {"cyclops", "cyclops smith", "cyclops drone"}, count = 120, rewards = {{type="exp", value=6750}, {type="gold", value=10800}, {type="points", value=4}}, minLevel = 8, maxLevel = 30},
            {name = "Low Humanoids", mobs = {"dark magician", "bandit", "smuggler", "amazon", "gladiator"}, count = 150, rewards = {{type="exp", value=3600}, {type="gold", value=5400}, {type="points", value=2}}, minLevel = 8, maxLevel = 30},
            {name = "Dwarf Dig", mobs = {"dwarf", "dwarf soldier", "dwarf guard"}, count = 135, rewards = {{type="exp", value=4500}, {type="gold", value=6480}, {type="points", value=2}}, minLevel = 8, maxLevel = 30},
            
            -- BRACKET 2: Level 30-50 (4-6 pts, 105-150 kills, 10.8k-21.6k gold)
            {name = "Barbarian Camp", mobs = {"barbarian headsplitter", "barbarian skullhunter", "barbarian brutetamer", "barbarian bloodwalker"}, count = 135, rewards = {{type="exp", value=12000}, {type="gold", value=14400}, {type="points", value=4}}, minLevel = 30, maxLevel = 50},
            {name = "Banuta Apes", mobs = {"kongra", "sibang", "merlkin"}, count = 120, rewards = {{type="exp", value=12000}, {type="gold", value=14400}, {type="points", value=4}}, minLevel = 30, maxLevel = 50},
            {name = "Dragon Hatchlings", mobs = {"dragon", "dragon hatchling"}, count = 135, rewards = {{type="exp", value=18000}, {type="gold", value=21600}, {type="points", value=6}}, minLevel = 30, maxLevel = 50},
            {name = "Pirate Plunder", mobs = {"pirate corsair", "pirate buccaneer", "pirate cutthroat", "pirate marauder"}, count = 135, rewards = {{type="exp", value=12000}, {type="gold", value=14400}, {type="points", value=4}}, minLevel = 30, maxLevel = 50},
            {name = "Easy Tomb", mobs = {"ghoul", "ghost", "demon skeleton", "skeleton warrior", "crypt shambler", "mummy", "scarab"}, count = 135, rewards = {{type="exp", value=13500}, {type="gold", value=16200}, {type="points", value=4}}, minLevel = 30, maxLevel = 50},
            {name = "Mutated Rats", mobs = {"mutated rat", "mutated bat"}, count = 120, rewards = {{type="exp", value=10500}, {type="gold", value=12600}, {type="points", value=4}}, minLevel = 30, maxLevel = 50},
            {name = "Easy Quara", mobs = {"quara predator scout", "quara pincher scout", "quara hydromancer scout"}, count = 150, rewards = {{type="exp", value=15000}, {type="gold", value=18000}, {type="points", value=6}}, minLevel = 30, maxLevel = 50},
            {name = "Killer Caimen", mobs = {"killer caiman"}, count = 120, rewards = {{type="exp", value=10500}, {type="gold", value=12600}, {type="points", value=4}}, minLevel = 30, maxLevel = 50},
            {name = "Ancient Scarab Tomb", mobs = {"ancient scarab"}, count = 105, rewards = {{type="exp", value=18000}, {type="gold", value=21600}, {type="points", value=6}}, minLevel = 30, maxLevel = 50},
            {name = "Elementals", mobs = {"earth elemental", "fire elemental", "water elemental"}, count = 105, rewards = {{type="exp", value=13500}, {type="gold", value=16200}, {type="points", value=4}}, minLevel = 30, maxLevel = 50},
            {name = "Bonelord Lair", mobs = {"bonelord", "elder bonelord"}, count = 120, rewards = {{type="exp", value=12000}, {type="gold", value=10800}, {type="points", value=4}}, minLevel = 30, maxLevel = 50},
            {name = "Haunted Treeling", mobs = {"haunted treeling"}, count = 105, rewards = {{type="exp", value=10500}, {type="gold", value=10800}, {type="points", value=4}}, minLevel = 30, maxLevel = 50},
            {name = "Deepling Shallows", mobs = {"deepling scout", "deepling brawler", "deepling worker"}, count = 135, rewards = {{type="exp", value=12000}, {type="gold", value=14400}, {type="points", value=4}}, minLevel = 30, maxLevel = 50},
            {name = "Hive Scouts", mobs = {"swarmer", "insectoid worker"}, count = 135, rewards = {{type="exp", value=12000}, {type="gold", value=14400}, {type="points", value=4}}, minLevel = 30, maxLevel = 50},
            
            -- BRACKET 3: Level 50-125 (3-5 pts, 90-135 kills, 14.4k-27k gold)
            {name = "All Cult", mobs = {"novice of the cult", "acolyte of the cult", "adept of the cult", "enlightened of the cult"}, count = 135, rewards = {{type="exp", value=18000}, {type="gold", value=14400}, {type="points", value=3}}, minLevel = 50, maxLevel = 125},
            {name = "Hard Tomb", mobs = {"bonebeast", "vampire", "necromancer", "giant spider"}, count = 120, rewards = {{type="exp", value=27000}, {type="gold", value=21600}, {type="points", value=4}}, minLevel = 50, maxLevel = 125},
            {name = "Any Djinn", mobs = {"efreet", "marid", "blue djinn", "green djinn"}, count = 105, rewards = {{type="exp", value=22500}, {type="gold", value=18000}, {type="points", value=4}}, minLevel = 50, maxLevel = 125},
            {name = "Bog Raiders", mobs = {"bog raider", "mutated tiger"}, count = 120, rewards = {{type="exp", value=18000}, {type="gold", value=14400}, {type="points", value=3}}, minLevel = 50, maxLevel = 125},
            {name = "Heroes", mobs = {"hero", "blood priest", "renegade knight", "vile grandmaster", "vicious squire"}, count = 90, rewards = {{type="exp", value=31500}, {type="gold", value=27000}, {type="points", value=5}}, minLevel = 50},
            {name = "Wyrm Storm", mobs = {"wyrm", "elder wyrm"}, count = 90, rewards = {{type="exp", value=31500}, {type="gold", value=27000}, {type="points", value=5}}, minLevel = 50},
            {name = "Nightmare Purge", mobs = {"nightmare", "nightmare scion"}, count = 105, rewards = {{type="exp", value=27000}, {type="gold", value=23400}, {type="points", value=5}}, minLevel = 50, maxLevel = 125},
            {name = "Sea Serpent Depths", mobs = {"sea serpent", "young sea serpent"}, count = 105, rewards = {{type="exp", value=22500}, {type="gold", value=18000}, {type="points", value=4}}, minLevel = 50, maxLevel = 125},
            {name = "Zombies & Stalkers", mobs = {"zombie", "nightstalker"}, count = 105, rewards = {{type="exp", value=18000}, {type="gold", value=14400}, {type="points", value=3}}, minLevel = 50, maxLevel = 125},
            {name = "Brimstone & Widows", mobs = {"brimstone bug", "wailing widow"}, count = 120, rewards = {{type="exp", value=15000}, {type="gold", value=14400}, {type="points", value=3}}, minLevel = 50, maxLevel = 125},
            {name = "Hellspawn Fire", mobs = {"hellspawn", "plaguesmith"}, count = 90, rewards = {{type="exp", value=31500}, {type="gold", value=27000}, {type="points", value=5}}, minLevel = 50, maxLevel = 125},
            {name = "Banshee Crypt", mobs = {"banshee", "braindeath"}, count = 90, rewards = {{type="exp", value=22500}, {type="gold", value=18000}, {type="points", value=4}}, minLevel = 50, maxLevel = 125},
            {name = "Souleater Feast", mobs = {"souleater"}, count = 90, rewards = {{type="exp", value=22500}, {type="gold", value=18000}, {type="points", value=4}}, minLevel = 50, maxLevel = 125},
            {name = "Worker Golems", mobs = {"worker golem", "war golem"}, count = 90, rewards = {{type="exp", value=22500}, {type="gold", value=18000}, {type="points", value=4}}, minLevel = 50, maxLevel = 125},
            {name = "Dragon Slayer", mobs = {"dragon"}, count = 105, rewards = {{type="exp", value=27000}, {type="gold", value=21600}, {type="points", value=4}}, minLevel = 50, maxLevel = 125},
            {name = "Hydra Heads", mobs = {"hydra"}, count = 90, rewards = {{type="exp", value=31500}, {type="gold", value=27000}, {type="points", value=5}}, minLevel = 50, maxLevel = 125},
            {name = "Deepling Depths", mobs = {"deepling warrior", "deepling spellsinger", "deepling guard"}, count = 105, rewards = {{type="exp", value=27000}, {type="gold", value=21600}, {type="points", value=4}}, minLevel = 50, maxLevel = 125},
            {name = "Hive Infestation", mobs = {"waspoid", "insectoid worker"}, count = 105, rewards = {{type="exp", value=22500}, {type="gold", value=18000}, {type="points", value=4}}, minLevel = 50, maxLevel = 125},
            
            -- BRACKET 4: Level 125+ (6-12 pts, 60-105 kills, 24k-48k gold) - No maxLevel so 225+ players see these as their "lower tier"
            {name = "Demon Slayer", mobs = {"demon"}, count = 75, rewards = {{type="exp", value=168750}, {type="gold", value=48000}, {type="points", value=10}}, minLevel = 125},
            {name = "DL & Frost", mobs = {"dragon lord", "frost dragon"}, count = 75, rewards = {{type="exp", value=135000}, {type="gold", value=36000}, {type="points", value=8}}, minLevel = 125},
            {name = "Lizard City", mobs = {"lizard chosen", "lizard zaogun", "lizard high guard", "lizard legionnaire"}, count = 105, rewards = {{type="exp", value=135000}, {type="gold", value=36000}, {type="points", value=8}}, minLevel = 125},
            {name = "Draken Walls", mobs = {"draken spellweaver", "draken warmaster"}, count = 90, rewards = {{type="exp", value=157500}, {type="gold", value=42000}, {type="points", value=9}}, minLevel = 125},
            {name = "Grim Reaper Harvest", mobs = {"grim reaper"}, count = 60, rewards = {{type="exp", value=168750}, {type="gold", value=48000}, {type="points", value=10}}, minLevel = 125},
            {name = "Hard Quara", mobs = {"quara predator", "quara hydromancer", "quara pincher", "quara constrictor"}, count = 105, rewards = {{type="exp", value=135000}, {type="gold", value=36000}, {type="points", value=8}}, minLevel = 125},
            {name = "Medusa Gaze", mobs = {"medusa", "serpent spawn"}, count = 75, rewards = {{type="exp", value=157500}, {type="gold", value=42000}, {type="points", value=9}}, minLevel = 125},
            {name = "Spectre Haunt", mobs = {"spectre", "betrayed wraith", "lost soul"}, count = 75, rewards = {{type="exp", value=144000}, {type="gold", value=36000}, {type="points", value=8}}, minLevel = 125},
            {name = "Hellfire Fighters", mobs = {"hellfire fighter", "massive fire elemental"}, count = 60, rewards = {{type="exp", value=157500}, {type="gold", value=42000}, {type="points", value=9}}, minLevel = 125},
            {name = "Necro Tower", mobs = {"necromancer", "warlock"}, count = 75, rewards = {{type="exp", value=157500}, {type="gold", value=42000}, {type="points", value=9}}, minLevel = 125},
            {name = "Destroyer Smash", mobs = {"destroyer", "diabolic imp"}, count = 75, rewards = {{type="exp", value=144000}, {type="gold", value=36000}, {type="points", value=8}}, minLevel = 125},
            {name = "Nightmare Isles", mobs = {"nightmare", "nightmare scion"}, count = 90, rewards = {{type="exp", value=126000}, {type="gold", value=30000}, {type="points", value=7}}, minLevel = 125},
            {name = "Deepling Stronghold", mobs = {"deepling elite", "deepling tyrant", "deepling master librarian"}, count = 75, rewards = {{type="exp", value=144000}, {type="gold", value=36000}, {type="points", value=8}}, minLevel = 125},
            {name = "Hive Soldiers", mobs = {"kollos", "spidris"}, count = 75, rewards = {{type="exp", value=135000}, {type="gold", value=36000}, {type="points", value=8}}, minLevel = 125},
            
            -- BRACKET 5: Level 225+ (14-22 pts, 30-50 kills, 50k-90k gold) - Expert tier
            {name = "Undead Dragon Bane", mobs = {"undead dragon"}, count = 35, rewards = {{type="exp", value=472500}, {type="gold", value=80000}, {type="points", value=18}}, minLevel = 225},
            {name = "Ghastly Dragon Hunt", mobs = {"ghastly dragon"}, count = 45, rewards = {{type="exp", value=405000}, {type="gold", value=70000}, {type="points", value=16}}, minLevel = 225},
            {name = "Juggernaut & Hellhound", mobs = {"juggernaut", "hellhound"}, count = 30, rewards = {{type="exp", value=540000}, {type="gold", value=90000}, {type="points", value=22}}, minLevel = 225},
            {name = "Fury & Demon", mobs = {"fury", "demon"}, count = 40, rewards = {{type="exp", value=472500}, {type="gold", value=80000}, {type="points", value=18}}, minLevel = 225},
            {name = "Dark Torturer Dungeon", mobs = {"dark torturer", "lost soul", "banshee"}, count = 35, rewards = {{type="exp", value=472500}, {type="gold", value=80000}, {type="points", value=18}}, minLevel = 225},
            {name = "Blightwalker Bane", mobs = {"blightwalker", "defiler", "son of verminor"}, count = 30, rewards = {{type="exp", value=495000}, {type="gold", value=85000}, {type="points", value=20}}, minLevel = 225},
            {name = "Behemoth Crush", mobs = {"behemoth"}, count = 40, rewards = {{type="exp", value=405000}, {type="gold", value=70000}, {type="points", value=16}}, minLevel = 225},
            {name = "Phantasm Nightmare", mobs = {"phantasm"}, count = 30, rewards = {{type="exp", value=472500}, {type="gold", value=80000}, {type="points", value=18}}, minLevel = 225},
            {name = "Hellfire & Hellhound", mobs = {"hellfire fighter", "hellhound"}, count = 30, rewards = {{type="exp", value=495000}, {type="gold", value=85000}, {type="points", value=20}}, minLevel = 225},
            {name = "Plaguesmith Forge", mobs = {"plaguesmith", "blightwalker"}, count = 30, rewards = {{type="exp", value=472500}, {type="gold", value=80000}, {type="points", value=18}}, minLevel = 225},
            {name = "Hive Core", mobs = {"spidris elite", "hive overseer"}, count = 30, rewards = {{type="exp", value=495000}, {type="gold", value=85000}, {type="points", value=20}}, minLevel = 225},
        },
        weekly = {
            -- Weekly Tasks - 5 clean level brackets, no overlap
            -- Points scale steeply with level to reward endgame players
            
            -- BRACKET 1: Level 8-30 (8-12 pts, 900-1200 kills, 27k-54k gold)
            {name = "Troll Champion", mobs = {"troll", "troll champion", "swamp troll"}, count = 1200, rewards = {{type="exp", value=36000}, {type="gold", value=27000}, {type="points", value=8}}, minLevel = 8, maxLevel = 30},
            {name = "Rotworm Queen", mobs = {"rotworm", "carrion worm"}, count = 1200, rewards = {{type="exp", value=36000}, {type="gold", value=27000}, {type="points", value=8}}, minLevel = 8, maxLevel = 30},
            {name = "Minotaur Army", mobs = {"minotaur", "minotaur archer", "minotaur guard", "minotaur mage"}, count = 1050, rewards = {{type="exp", value=54000}, {type="gold", value=43200}, {type="points", value=10}}, minLevel = 8, maxLevel = 30},
            {name = "Orc Warlord", mobs = {"orc", "orc spearman", "orc warrior", "orc berserker", "orc leader"}, count = 1050, rewards = {{type="exp", value=54000}, {type="gold", value=43200}, {type="points", value=10}}, minLevel = 8, maxLevel = 30},
            {name = "Elf Village", mobs = {"elf", "elf scout", "elf arcanist"}, count = 1050, rewards = {{type="exp", value=45000}, {type="gold", value=32400}, {type="points", value=8}}, minLevel = 8, maxLevel = 30},
            {name = "Dwarf Kingdom", mobs = {"dwarf", "dwarf soldier", "dwarf guard", "dwarf geomancer"}, count = 1050, rewards = {{type="exp", value=54000}, {type="gold", value=43200}, {type="points", value=10}}, minLevel = 8, maxLevel = 30},
            {name = "Cyclops Behemoth", mobs = {"cyclops", "cyclops smith", "cyclops drone"}, count = 900, rewards = {{type="exp", value=67500}, {type="gold", value=54000}, {type="points", value=12}}, minLevel = 8, maxLevel = 30},
            
            -- BRACKET 2: Level 30-50 (12-20 pts, 750-1050 kills, 54k-90k gold)
            {name = "Barbarian Horde", mobs = {"barbarian headsplitter", "barbarian skullhunter", "barbarian brutetamer", "barbarian bloodwalker"}, count = 1050, rewards = {{type="exp", value=54000}, {type="gold", value=72000}, {type="points", value=16}}, minLevel = 30, maxLevel = 50},
            {name = "Banuta Apes", mobs = {"kongra", "sibang", "merlkin"}, count = 750, rewards = {{type="exp", value=54000}, {type="gold", value=72000}, {type="points", value=16}}, minLevel = 30, maxLevel = 50},
            {name = "Ancient Scarabs", mobs = {"ancient scarab"}, count = 900, rewards = {{type="exp", value=66000}, {type="gold", value=90000}, {type="points", value=20}}, minLevel = 30, maxLevel = 50},
            {name = "Mutated Lab", mobs = {"mutated rat", "mutated tiger", "mutated bat"}, count = 1050, rewards = {{type="exp", value=48000}, {type="gold", value=64800}, {type="points", value=16}}, minLevel = 30, maxLevel = 50},
            {name = "Tortoise Shells", mobs = {"tortoise", "thornback tortoise"}, count = 900, rewards = {{type="exp", value=42000}, {type="gold", value=54000}, {type="points", value=12}}, minLevel = 30, maxLevel = 50},
            {name = "Beetle Crush", mobs = {"lancer beetle", "wailing widow"}, count = 900, rewards = {{type="exp", value=48000}, {type="gold", value=64800}, {type="points", value=16}}, minLevel = 30, maxLevel = 50},
            {name = "Dragon Slayer", mobs = {"dragon", "dragon hatchling"}, count = 750, rewards = {{type="exp", value=66000}, {type="gold", value=90000}, {type="points", value=20}}, minLevel = 30, maxLevel = 50},
            {name = "Pirate Fleet", mobs = {"pirate corsair", "pirate buccaneer", "pirate cutthroat", "pirate marauder"}, count = 900, rewards = {{type="exp", value=48000}, {type="gold", value=64800}, {type="points", value=16}}, minLevel = 30, maxLevel = 50},
            {name = "Deepling Tide", mobs = {"deepling scout", "deepling brawler", "deepling worker"}, count = 1050, rewards = {{type="exp", value=48000}, {type="gold", value=64800}, {type="points", value=16}}, minLevel = 30, maxLevel = 50},
            {name = "Hive Swarm", mobs = {"swarmer", "insectoid worker"}, count = 1050, rewards = {{type="exp", value=48000}, {type="gold", value=64800}, {type="points", value=16}}, minLevel = 30, maxLevel = 50},
            
            -- BRACKET 3: Level 50-125 (10-16 pts, 600-900 kills, 72k-126k gold)
            {name = "Giant Spiders", mobs = {"giant spider"}, count = 750, rewards = {{type="exp", value=37500}, {type="gold", value=81000}, {type="points", value=12}}, minLevel = 50, maxLevel = 125},
            {name = "Heroism", mobs = {"hero", "blood priest", "renegade knight", "vile grandmaster", "vicious squire"}, count = 750, rewards = {{type="exp", value=45000}, {type="gold", value=108000}, {type="points", value=14}}, minLevel = 50},
            {name = "Djinn Fortress", mobs = {"efreet", "marid", "blue djinn", "green djinn"}, count = 750, rewards = {{type="exp", value=37500}, {type="gold", value=81000}, {type="points", value=12}}, minLevel = 50, maxLevel = 125},
            {name = "Wyrm Electricity", mobs = {"wyrm", "elder wyrm"}, count = 675, rewards = {{type="exp", value=52500}, {type="gold", value=117000}, {type="points", value=14}}, minLevel = 50},
            {name = "Nightmare Slaughter", mobs = {"nightmare", "nightmare scion"}, count = 750, rewards = {{type="exp", value=45000}, {type="gold", value=108000}, {type="points", value=14}}, minLevel = 50, maxLevel = 125},
            {name = "Sea Serpent Hunt", mobs = {"sea serpent", "young sea serpent"}, count = 600, rewards = {{type="exp", value=45000}, {type="gold", value=99000}, {type="points", value=12}}, minLevel = 50, maxLevel = 125},
            {name = "Souleater Purge", mobs = {"souleater"}, count = 600, rewards = {{type="exp", value=42000}, {type="gold", value=90000}, {type="points", value=12}}, minLevel = 50, maxLevel = 125},
            {name = "Quara Invasion", mobs = {"quara predator", "quara hydromancer", "quara pincher"}, count = 750, rewards = {{type="exp", value=48000}, {type="gold", value=108000}, {type="points", value=14}}, minLevel = 50, maxLevel = 125},
            {name = "Ice Cold", mobs = {"ice golem", "crystal spider"}, count = 900, rewards = {{type="exp", value=45000}, {type="gold", value=90000}, {type="points", value=12}}, minLevel = 50, maxLevel = 125},
            {name = "Wyvern Nest", mobs = {"wyvern"}, count = 900, rewards = {{type="exp", value=37500}, {type="gold", value=72000}, {type="points", value=10}}, minLevel = 50, maxLevel = 125},
            {name = "Lich Tombs", mobs = {"lich", "banshee"}, count = 600, rewards = {{type="exp", value=42000}, {type="gold", value=90000}, {type="points", value=12}}, minLevel = 50, maxLevel = 125},
            {name = "Hydra Heads", mobs = {"hydra"}, count = 600, rewards = {{type="exp", value=67500}, {type="gold", value=126000}, {type="points", value=16}}, minLevel = 50, maxLevel = 125},
            {name = "Hellspawn & Plague", mobs = {"hellspawn", "plaguesmith"}, count = 600, rewards = {{type="exp", value=60000}, {type="gold", value=117000}, {type="points", value=14}}, minLevel = 50, maxLevel = 125},
            {name = "Bog & Mutant", mobs = {"bog raider", "mutated tiger"}, count = 750, rewards = {{type="exp", value=42000}, {type="gold", value=81000}, {type="points", value=12}}, minLevel = 50, maxLevel = 125},
            {name = "Deepling Abyss", mobs = {"deepling warrior", "deepling spellsinger", "deepling guard"}, count = 750, rewards = {{type="exp", value=48000}, {type="gold", value=108000}, {type="points", value=14}}, minLevel = 50, maxLevel = 125},
            {name = "Hive Purge", mobs = {"waspoid", "insectoid worker"}, count = 750, rewards = {{type="exp", value=42000}, {type="gold", value=81000}, {type="points", value=12}}, minLevel = 50, maxLevel = 125},
            
            -- BRACKET 4: Level 125+ (20-40 pts, 400-750 kills, 96k-180k gold) - No maxLevel so 225+ players see these as their "lower tier"
            {name = "Dragon Lord Hunt", mobs = {"dragon lord", "frost dragon"}, count = 600, rewards = {{type="exp", value=225000}, {type="gold", value=120000}, {type="points", value=22}}, minLevel = 125},
            {name = "Lizard Chosen", mobs = {"lizard chosen", "lizard zaogun", "lizard high guard"}, count = 675, rewards = {{type="exp", value=202500}, {type="gold", value=108000}, {type="points", value=22}}, minLevel = 125},
            {name = "Draken Walls", mobs = {"draken warmaster", "draken spellweaver"}, count = 600, rewards = {{type="exp", value=270000}, {type="gold", value=144000}, {type="points", value=24}}, minLevel = 125},
            {name = "Warlock Study", mobs = {"warlock"}, count = 450, rewards = {{type="exp", value=292500}, {type="gold", value=168000}, {type="points", value=28}}, minLevel = 125},
            {name = "Spectre Haunting", mobs = {"spectre", "betrayed wraith", "lost soul"}, count = 525, rewards = {{type="exp", value=247500}, {type="gold", value=132000}, {type="points", value=24}}, minLevel = 125},
            {name = "Destroyer Smash", mobs = {"destroyer", "diabolic imp"}, count = 600, rewards = {{type="exp", value=225000}, {type="gold", value=120000}, {type="points", value=24}}, minLevel = 125},
            {name = "Medusa Glare", mobs = {"medusa", "serpent spawn", "hydra"}, count = 600, rewards = {{type="exp", value=292500}, {type="gold", value=168000}, {type="points", value=28}}, minLevel = 125},
            {name = "Grim Reaper", mobs = {"grim reaper"}, count = 525, rewards = {{type="exp", value=292500}, {type="gold", value=180000}, {type="points", value=28}}, minLevel = 125},
            {name = "Hellfire Inferno", mobs = {"hellfire fighter", "massive fire elemental"}, count = 450, rewards = {{type="exp", value=270000}, {type="gold", value=156000}, {type="points", value=26}}, minLevel = 125},
            {name = "Necro & Warlock", mobs = {"necromancer", "warlock"}, count = 525, rewards = {{type="exp", value=247500}, {type="gold", value=132000}, {type="points", value=24}}, minLevel = 125},
            {name = "Sea Serpent Abyss", mobs = {"sea serpent"}, count = 600, rewards = {{type="exp", value=202500}, {type="gold", value=96000}, {type="points", value=20}}, minLevel = 125},
            {name = "Behemoth Crush", mobs = {"behemoth"}, count = 750, rewards = {{type="exp", value=225000}, {type="gold", value=120000}, {type="points", value=22}}, minLevel = 125},
            {name = "Deepling Fortress", mobs = {"deepling elite", "deepling tyrant", "deepling master librarian"}, count = 525, rewards = {{type="exp", value=247500}, {type="gold", value=132000}, {type="points", value=24}}, minLevel = 125},
            {name = "Hive Warzone", mobs = {"kollos", "spidris"}, count = 525, rewards = {{type="exp", value=247500}, {type="gold", value=132000}, {type="points", value=24}}, minLevel = 125},
            
            -- BRACKET 5: Level 225+ (45-80 pts, 150-400 kills, 150k-300k gold) - Expert tier
            {name = "Ghastly Dragons", mobs = {"ghastly dragon"}, count = 400, rewards = {{type="exp", value=645000}, {type="gold", value=250000}, {type="points", value=55}}, minLevel = 225},
            {name = "Dark Torturers", mobs = {"dark torturer", "lost soul", "demon"}, count = 250, rewards = {{type="exp", value=705000}, {type="gold", value=280000}, {type="points", value=60}}, minLevel = 225},
            {name = "Demon Slayer", mobs = {"demon"}, count = 400, rewards = {{type="exp", value=765000}, {type="gold", value=300000}, {type="points", value=65}}, minLevel = 225},
            {name = "Juggernaut Charge", mobs = {"juggernaut", "hellhound"}, count = 200, rewards = {{type="exp", value=825000}, {type="gold", value=300000}, {type="points", value=75}}, minLevel = 225},
            {name = "Undead Dragons", mobs = {"undead dragon"}, count = 150, rewards = {{type="exp", value=877500}, {type="gold", value=300000}, {type="points", value=80}}, minLevel = 225},
            {name = "Blightwalker Purge", mobs = {"blightwalker", "defiler", "son of verminor"}, count = 250, rewards = {{type="exp", value=705000}, {type="gold", value=270000}, {type="points", value=60}}, minLevel = 225},
            {name = "Fury Rage", mobs = {"fury", "juggernaut"}, count = 150, rewards = {{type="exp", value=765000}, {type="gold", value=300000}, {type="points", value=65}}, minLevel = 225},
            {name = "Phantasm Realm", mobs = {"phantasm"}, count = 200, rewards = {{type="exp", value=585000}, {type="gold", value=200000}, {type="points", value=50}}, minLevel = 225},
            {name = "Hellfire & Hellhound", mobs = {"hellfire fighter", "hellhound"}, count = 200, rewards = {{type="exp", value=765000}, {type="gold", value=300000}, {type="points", value=65}}, minLevel = 225},
            {name = "Behemoth Army", mobs = {"behemoth"}, count = 400, rewards = {{type="exp", value=540000}, {type="gold", value=150000}, {type="points", value=45}}, minLevel = 225},
            {name = "Hive Extermination", mobs = {"spidris elite", "hive overseer"}, count = 200, rewards = {{type="exp", value=765000}, {type="gold", value=300000}, {type="points", value=65}}, minLevel = 225},
        }
    }
}

-- Simple Linear Congruential Generator for seeded random numbers
local function seededRandom(seed)
    local a = 1664525
    local c = 1013904223
    local m = 4294967296
    seed = (a * seed + c) % m
    return seed
end

-- Shuffle table based on seed
local function shuffleTable(tbl, seed)
    local newTbl = {unpack(tbl)}
    local n = #newTbl
    local randomState = seed
    
    for i = n, 2, -1 do
        randomState = seededRandom(randomState)
        local j = (randomState % i) + 1
        newTbl[i], newTbl[j] = newTbl[j], newTbl[i]
    end
    return newTbl
end

-- Cache variables
local cachedDailyPool = {seed = 0, indices = {}}
local cachedWeeklyPool = {seed = 0, indices = {}}
local DISPLAY_LIMIT = 10

-- Helpers to get tasks based on date/week
local function getDailyPool()
    local seed = tonumber(os.date("%Y%m%d"))
    if cachedDailyPool.seed == seed then
        return cachedDailyPool.indices, seed
    end

    local indices = {}
    for i=1, #config.pool.daily do indices[i] = i end
    
    local shuffled = shuffleTable(indices, seed)
    -- Return ALL shuffled indices, filtering happens per-player
    
    cachedDailyPool = {seed = seed, indices = shuffled}
    return shuffled, seed
end

local function getWeeklyPool()
    local seed = tonumber(os.date("%Y%W")) -- Year + Week Number
    if cachedWeeklyPool.seed == seed then
        return cachedWeeklyPool.indices, seed
    end

    local indices = {}
    for i=1, #config.pool.weekly do indices[i] = i end
    
    local shuffled = shuffleTable(indices, seed)
    -- Return ALL shuffled indices, filtering happens per-player
    
    cachedWeeklyPool = {seed = seed, indices = shuffled}
    return shuffled, seed
end

-- Storage Helpers
local function getStorage(p, key) return math.max(0, p:getStorageValue(key)) end
local function setStorage(p, key, val) p:setStorageValue(key, val) end


local function updateCompletions(player)
    local dailySeed = tonumber(os.date("%Y%m%d"))
    local weeklySeed = tonumber(os.date("%Y%W"))
    
    -- Reset Daily count if day changed (allows starting 3 new tasks)
    if getStorage(player, config.storage.daily_seed) ~= dailySeed then
        setStorage(player, config.storage.daily_seed, dailySeed)
        setStorage(player, config.storage.daily_count, 0)
        -- Reset daily task states so new rotation is available
        for i = 1, #config.pool.daily do
            setStorage(player, config.storage.state_start + 100 + i, 0)
            setStorage(player, config.storage.kills_start + 100 + i, 0)
            setStorage(player, Storage.TaskSystem.ExtendedBase + 100 + i, 0) -- Reset extended flag
        end
    end
    
    -- Reset Weekly count if week changed (allows starting 3 new tasks)
    if getStorage(player, config.storage.weekly_seed) ~= weeklySeed then
        setStorage(player, config.storage.weekly_seed, weeklySeed)
        setStorage(player, config.storage.weekly_count, 0)
        -- Reset weekly task states so new rotation is available
        for i = 1, #config.pool.weekly do
            setStorage(player, config.storage.state_start + 200 + i, 0)
            setStorage(player, config.storage.kills_start + 200 + i, 0)
            setStorage(player, Storage.TaskSystem.ExtendedBase + 200 + i, 0) -- Reset extended flag
        end
    end
end

-- Helper function to get experience stage multiplier based on player level
local function getExpStageMultiplier(playerLevel)
    if playerLevel <= 20 then
        return 5
    elseif playerLevel <= 40 then
        return 4
    elseif playerLevel <= 60 then
        return 3
    elseif playerLevel <= 80 then
        return 2.5
    elseif playerLevel <= 120 then
        return 2
    elseif playerLevel <= 200 then
        return 1.5
    else
        return 1
    end
end

-- Helper function to apply stage multiplier to rewards for display
local function getScaledRewards(rewards, playerLevel)
    local stageMultiplier = getExpStageMultiplier(playerLevel)
    local scaledRewards = {}
    for _, r in ipairs(rewards) do
        if r.type == "exp" then
            table.insert(scaledRewards, {type = r.type, value = math.floor(r.value * stageMultiplier)})
        elseif r.type == "gold" then
            -- Gold values are flat per-task, no stage scaling
            table.insert(scaledRewards, {type = r.type, value = r.value})
        else
            table.insert(scaledRewards, r)
        end
    end
    return scaledRewards
end

local function sendData(player)
    updateCompletions(player)
    
    -- Get reward preference (0 = EXP, 1 = Gold)
    local rewardPref = getStorage(player, config.storage.reward_preference)
    if rewardPref ~= 1 then rewardPref = 0 end -- Default to EXP
    
    -- Get Paw & Fur points
    local pawFurPoints = math.max(0, player:getStorageValue(Storage.KillingInTheNameOf.Points))
    
    local payload = {
        action = "sync",
        points = getStorage(player, config.storage.points),
        pawFurPoints = pawFurPoints,
        rewardPreference = rewardPref, -- 0 = EXP, 1 = Gold
        expToGoldRatio = config.expToGoldRatio,
        info = {
            dailyCount = getStorage(player, config.storage.daily_count),
            dailyLimit = config.limits.daily,
            weeklyCount = getStorage(player, config.storage.weekly_count),
            weeklyLimit = config.limits.weekly
        },
        tasks = {},
        shop = { items = {} },
        extendTasks = {}  -- Active tasks that can be extended
    }
    
    -- Populate Shop
    for id, item in ipairs(config.shop) do
        local purchased = false
        -- Check if this is a permanent upgrade that's already purchased
        if item.storageKey and Storage.TaskSystem[item.storageKey] then
            purchased = player:getStorageValue(Storage.TaskSystem[item.storageKey]) == 1
        end
        table.insert(payload.shop.items, {
            id = id, 
            name = item.name, 
            desc = item.desc, 
            cost = item.cost, 
            itemId = item.clientId, 
            category = item.category or "Potions",
            purchased = purchased
        })
    end
    
    -- Function to populate tasks
    local function addTasksToPayload(poolIndices, categoryPool, categoryName)
        local playerLevel = player:getLevel()
        
        -- First pass: collect all eligible tasks with weighted scores for variety
        local priorityTasks = {}  -- Active/claimable tasks (always shown)
        local eligibleTasks = {}  -- All other eligible tasks with weight scores
        
        for _, index in ipairs(poolIndices) do
            local t = categoryPool[index]
            
            -- Filter by Level
            local meetsMin = (not t.minLevel) or (playerLevel >= t.minLevel)
            local meetsMax = (not t.maxLevel) or (playerLevel <= t.maxLevel)
            
            -- Check if player has an active or claimable task (should always show regardless of level)
            local id = (categoryName == "Daily" and 100 or 200) + index
            local currentState = getStorage(player, config.storage.state_start + id)
            local hasActiveOrClaimable = (currentState == 1 or currentState == 2)
            
            if t and (hasActiveOrClaimable or (meetsMin and meetsMax)) then
                if hasActiveOrClaimable then
                    table.insert(priorityTasks, {
                        index = index,
                        task = t,
                        state = currentState
                    })
                else
                    -- Weight: higher minLevel tasks get a better (lower) score
                    -- This gives them a soft advantage without locking out variety
                    -- Tasks close to player level score best, much lower tasks score worse
                    local taskMin = t.minLevel or 1
                    -- How far below the player's level is this task's entry point?
                    -- 0 = perfect match, bigger = further below player level
                    local levelGap = math.max(0, playerLevel - taskMin)
                    -- Every 30 levels below the player costs 1 penalty point
                    -- So a level 150 player: expert (gap 0-20) scores 0, upper (gap 50-70) scores ~2, mid (gap 70-100) scores ~3
                    local penalty = math.floor(levelGap / 30)
                    
                    -- Tasks with no maxLevel are designed to be available at all levels
                    -- Give them zero penalty so they always compete fairly in the pool
                    if not t.maxLevel then
                        penalty = 0
                    end
                    
                    table.insert(eligibleTasks, {
                        index = index,
                        task = t,
                        state = currentState,
                        penalty = penalty
                    })
                end
            end
        end
        
        -- The pool is already shuffled by the daily/weekly seed, so tasks within
        -- the same penalty band are in random (but deterministic) order.
        -- Stable sort by penalty so the seed-based shuffle provides variety within tiers.
        table.sort(eligibleTasks, function(a, b) return a.penalty < b.penalty end)
        
        -- Build final selection: priority tasks first, then top eligible
        local selectedTasks = {}
        
        for _, entry in ipairs(priorityTasks) do
            table.insert(selectedTasks, entry)
        end
        
        local slotsRemaining = DISPLAY_LIMIT - #selectedTasks
        
        -- Expert tier split: level 225+ players get 5 expert (minLevel>=225) tasks
        -- and 5 from the lower tier (125-224 range) for solo-friendly variety
        local EXPERT_LEVEL = 225
        local EXPERT_SLOTS = 5
        
        if playerLevel >= EXPERT_LEVEL then
            local expertTasks = {}
            local lowerTasks = {}
            
            for _, entry in ipairs(eligibleTasks) do
                local taskMin = entry.task.minLevel or 1
                if taskMin >= EXPERT_LEVEL then
                    table.insert(expertTasks, entry)
                else
                    table.insert(lowerTasks, entry)
                end
            end
            
            -- Fill expert slots first (up to EXPERT_SLOTS)
            local expertAdded = 0
            for i = 1, math.min(EXPERT_SLOTS, #expertTasks, slotsRemaining) do
                table.insert(selectedTasks, expertTasks[i])
                expertAdded = expertAdded + 1
            end
            
            -- Fill remaining slots with lower tier tasks
            local lowerSlots = slotsRemaining - expertAdded
            for i = 1, math.min(lowerSlots, #lowerTasks) do
                table.insert(selectedTasks, lowerTasks[i])
            end
            
            -- If we still have empty slots, backfill from whichever pool has leftovers
            local totalAdded = #selectedTasks - #priorityTasks
            if totalAdded < slotsRemaining then
                local remaining = slotsRemaining - totalAdded
                -- Try more expert tasks
                for i = expertAdded + 1, math.min(expertAdded + remaining, #expertTasks) do
                    table.insert(selectedTasks, expertTasks[i])
                    remaining = remaining - 1
                end
                -- Then more lower tasks
                local lowerAdded = math.min(lowerSlots, #lowerTasks)
                for i = lowerAdded + 1, math.min(lowerAdded + remaining, #lowerTasks) do
                    table.insert(selectedTasks, lowerTasks[i])
                end
            end
        else
            for i = 1, math.min(slotsRemaining, #eligibleTasks) do
                table.insert(selectedTasks, eligibleTasks[i])
            end
        end
        
        -- Second pass: add selected tasks to payload (up to DISPLAY_LIMIT)
        for i = 1, math.min(DISPLAY_LIMIT, #selectedTasks) do
            local entry = selectedTasks[i]
            local index = entry.index
            local t = entry.task
            
            local id = (categoryName == "Daily" and 100 or 200) + index
            local state = getStorage(player, config.storage.state_start + id)
            local kills = getStorage(player, config.storage.kills_start + id)
            local isExtended = getStorage(player, Storage.TaskSystem.ExtendedBase + id) == 1
            
            -- Calculate actual target (doubled if extended)
            local actualTarget = isExtended and (t.count * config.extend.multiplier) or t.count
            
            -- Construct outfit list (max 2)
            local outfits = {}
            for j = 1, math.min(2, #t.mobs) do
                local mType = MonsterType(t.mobs[j])
                if mType then
                    local o = mType:getOutfit()
                    table.insert(outfits, {type=o.lookType, head=o.lookHead, body=o.lookBody, legs=o.lookLegs, feet=o.lookFeet})
                end
            end
            
            -- Build monster list for description
            local mobsDesc = table.concat(t.mobs, ", ")
            
            table.insert(payload.tasks, {
                id = id,
                name = t.name .. (isExtended and " (Extended)" or ""),
                cat = categoryName,
                desc = "Kill " .. actualTarget .. " " .. mobsDesc,
                current = kills,
                target = actualTarget,
                state = state, -- 0=New, 1=Active, 2=Claimable, 3=Done
                outfits = outfits,
                rewards = getScaledRewards(t.rewards, playerLevel),
                repeatable = true,
                extended = isExtended
            })
            
            -- Add to extendTasks if active and not already extended
            if state == 1 and not isExtended then
                local progress = (kills / t.count) * 100
                local canExtend = progress < config.extend.maxProgress
                table.insert(payload.extendTasks, {
                    id = id,
                    name = t.name,
                    cat = categoryName,
                    current = kills,
                    target = t.count,
                    newTarget = t.count * config.extend.multiplier,
                    progress = math.floor(progress),
                    canExtend = canExtend,
                    cost = config.extend.cost,
                    outfits = outfits,
                    rewards = getScaledRewards(t.rewards, playerLevel)
                })
            end
        end
    end
    
    local dailyIndices = getDailyPool()
    local weeklyIndices = getWeeklyPool()
    
    addTasksToPayload(dailyIndices, config.pool.daily, "Daily")
    addTasksToPayload(weeklyIndices, config.pool.weekly, "Weekly")
    
    -- ========== KILLING IN THE NAME OF (Paw & Fur) ==========
    if tasks and player:getStorageValue(Storage.KillingInTheNameOf.Join) >= 0 then
        local started = player:getStartedTasks() or {}
        local available = player:getTasks() or {}
        
        for taskId, t in pairs(tasks) do
            local questStorage = player:getStorageValue(Storage.KillingInTheNameOf.QuestStorageBase + taskId)
            local kills = math.max(0, player:getStorageValue(Storage.KillingInTheNameOf.KillsStorageBase + taskId))
            local kitnofState = 0 -- Not started
            
            if questStorage == 1 then
                kitnofState = (kills >= t.killsRequired) and 2 or 1 -- Claimable or Active
            elseif questStorage >= 2 or (t.norepeatable and player:getStorageValue(Storage.KillingInTheNameOf.RepeatStorageBase + taskId) > 0) then
                kitnofState = 3 -- Done
            end
            
            local showTask = isInArray(started, taskId) or isInArray(available, taskId) or kitnofState > 0
            if showTask then
                local kitnofOutfits = {}
                if t.creatures then
                    for _, mob in ipairs(t.creatures) do
                        local mType = MonsterType(mob)
                        if mType then
                            local o = mType:getOutfit()
                            table.insert(kitnofOutfits, {type=o.lookType, head=o.lookHead, body=o.lookBody, legs=o.lookLegs, feet=o.lookFeet})
                        end
                    end
                end
                
                table.insert(payload.tasks, {
                    id = 1000 + taskId,
                    name = t.name or t.raceName or "Unknown",
                    cat = "Paw & Fur",
                    desc = string.format("Kill %d %s", t.killsRequired, t.raceName or "monsters"),
                    current = kills,
                    target = t.killsRequired,
                    state = kitnofState,
                    outfits = kitnofOutfits,
                    rewards = {{type="points", value=1}}, -- Basic reward display
                    repeatable = not t.norepeatable
                })
            end
        end
    end
    -- ========== END KITNOF ==========
    
    local data = json.encode(payload)
    if #data > 65000 then print("[TaskSystem] Payload too large!") return end
    player:sendExtendedOpcode(OPCODE, data)
end

-- ========== KILL EVENT (Same logic as KITNOF kills.lua) ==========
local recentKills = {}

-- Combat activity tracking for anti-leech
local recentCombatActivity = {} -- [playerGuid] = timestamp of last combat action
local COMBAT_ACTIVITY_WINDOW = 10000 -- 10 seconds - must have attacked within this window

-- Configuration
local DAMAGE_THRESHOLD = 0.10 -- 10% damage required for shared credit
local PARTY_SHARE_RANGE = 30 -- Distance in sqm for party credit
local SOLO_KILL_THRESHOLD = 0.51 -- 51% damage = exclusive credit
local PARTY_MIN_DAMAGE = 0.05 -- 1% minimum damage for party credit (anti-leech)

-- Helper: Check if player has recent combat activity
local function hasRecentCombatActivity(playerGuid)
    local lastActivity = recentCombatActivity[playerGuid]
    if not lastActivity then return false end
    return (os.mtime() - lastActivity) <= COMBAT_ACTIVITY_WINDOW
end

-- Helper: Record combat activity for a player
local function recordCombatActivity(playerGuid)
    recentCombatActivity[playerGuid] = os.mtime()
end

-- Helper: Check if player is within party level range for task sharing
-- Uses same formula as experience sharing: minLevel = ceil(highestLevel * 2 / 3)
local PARTY_LEVEL_RANGE_ENABLED = true -- Enable level range check (same as exp share)

local function isWithinPartyLevelRange(player, party)
    if not PARTY_LEVEL_RANGE_ENABLED then return true end
    
    -- Find highest level in party
    local highestLevel = party:getLeader():getLevel()
    for _, member in ipairs(party:getMembers()) do
        if member:getLevel() > highestLevel then
            highestLevel = member:getLevel()
        end
    end
    
    -- Calculate minimum level (same formula as C++ party.cpp)
    local minLevel = math.ceil((highestLevel * 2) / 3)
    
    return player:getLevel() >= minLevel
end

-- Build creature-to-task mapping for Daily/Weekly tasks
local creatureToTaskPool = {}
for i, t in ipairs(config.pool.daily) do
    for _, mob in ipairs(t.mobs) do
        local lowerName = mob:lower()
        creatureToTaskPool[lowerName] = creatureToTaskPool[lowerName] or {}
        table.insert(creatureToTaskPool[lowerName], {id = 100 + i, task = t})
    end
end
for i, t in ipairs(config.pool.weekly) do
    for _, mob in ipairs(t.mobs) do
        local lowerName = mob:lower()
        creatureToTaskPool[lowerName] = creatureToTaskPool[lowerName] or {}
        table.insert(creatureToTaskPool[lowerName], {id = 200 + i, task = t})
    end
end

local function giveTaskSystemCredit(player, targetName, targetId)
    local playerGuid = player:getGuid()
    
    -- Deduplication check
    if not recentKills[playerGuid] then
        recentKills[playerGuid] = {}
    end
    
    if recentKills[playerGuid][targetId] then
        return
    end
    
    recentKills[playerGuid][targetId] = true
    
    addEvent(function(guid, tid)
        if recentKills[guid] then
            recentKills[guid][tid] = nil
            if not next(recentKills[guid]) then
                recentKills[guid] = nil
            end
        end
    end, 1000, playerGuid, targetId)
    
    local needsSync = false
    
    -- Process Daily/Weekly task credit
    local taskEntries = creatureToTaskPool[targetName] or {}
    for _, entry in ipairs(taskEntries) do
        local id = entry.id
        local t = entry.task
        
        if getStorage(player, config.storage.state_start + id) == 1 then -- Active
            -- Level Validation - Only check minLevel for kill credit
            -- Players who outlevel a task (exceed maxLevel) can still complete it
            -- but they must meet the minimum level requirement
            local playerLevel = player:getLevel()
            local meetsMin = (not t.minLevel) or (playerLevel >= t.minLevel)
            
            if not meetsMin then
                -- Player is below minimum level (shouldn't happen normally)
                setStorage(player, config.storage.state_start + id, 0)
                setStorage(player, config.storage.kills_start + id, 0)
                player:sendTextMessage(MESSAGE_STATUS_WARNING, "Task " .. t.name .. " cancelled: level requirement no longer met.")
                needsSync = true
            else
                local kills = getStorage(player, config.storage.kills_start + id) + 1
                setStorage(player, config.storage.kills_start + id, kills)
                
                -- Check if task is extended (doubled target)
                local isExtended = getStorage(player, Storage.TaskSystem.ExtendedBase + id) == 1
                local targetCount = isExtended and (t.count * config.extend.multiplier) or t.count
                local taskName = isExtended and (t.name .. " (Extended)") or t.name
                
                if kills >= targetCount then
                    setStorage(player, config.storage.state_start + id, 2) -- Claimable
                    player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "Task completed: " .. taskName)
                else
                    player:sendTextMessage(MESSAGE_STATUS_CONSOLE_ORANGE, string.format("[%s] %d/%d", taskName, kills, targetCount))
                end
                needsSync = true
            end
        end
    end
    
    if needsSync then
        sendData(player)
    end
end

local ev = CreatureEvent("ModernTaskKill")
function ev.onKill(player, target)
    if not player then return true end
    if not target:isMonster() or target:getMaster() then return true end
    
    local targetId = target:getId()
    local targetName = target:getName():lower()
    
    -- Damage Calculation (same as KITNOF)
    local targetMaxHealth = target:getMaxHealth()
    local damageMap = target:getDamageMap()
    
    local eligiblePlayers = {}
    local killerDamage = 0
    
    -- NOTE: damageMap is keyed by player:getId() (runtime ID), not getGuid() (database ID)
    local killerRuntimeId = player:getId()
    if damageMap[killerRuntimeId] then
        killerDamage = damageMap[killerRuntimeId].total / targetMaxHealth
    end
    
    -- Record combat activity for all players who dealt damage (using GUID for storage)
    for attackerId, _ in pairs(damageMap) do
        local attacker = Player(attackerId)
        if attacker then
            recordCombatActivity(attacker:getGuid())
        end
    end
    
    -- IMPORTANT: Killer ALWAYS gets credit
    eligiblePlayers[player:getGuid()] = player
    
    -- Check if killer is in a party - if so, apply party sharing rules
    if player:getParty() then
        local party = player:getParty()
        local members = party:getMembers()
        members[#members + 1] = party:getLeader()
        
        local targetPos = target:getPosition()
        for _, member in ipairs(members) do
            -- Skip the killer (already added above)
            if member and member:getGuid() ~= player:getGuid() then
                if member:getPosition():getDistance(targetPos) <= PARTY_SHARE_RANGE then
                    local memberGuid = member:getGuid()
                    local memberRuntimeId = member:getId()
                    local memberDamage = damageMap[memberRuntimeId]
                    local damagePercent = memberDamage and (memberDamage.total / targetMaxHealth) or 0
                    
                    -- Anti-leech checks for party members (not the killer):
                    -- 1. Must be within party level range (same as exp share)
                    local withinLevelRange = isWithinPartyLevelRange(member, party)
                    
                    -- 2. Must meet EITHER damage OR activity condition:
                    --    a. Dealt at least minimum damage to this monster, OR
                    --    b. Has recent combat activity (attacked something recently)
                    local hasMinDamage = damagePercent >= PARTY_MIN_DAMAGE
                    local hasRecentCombat = hasRecentCombatActivity(memberGuid)
                    
                    if withinLevelRange and (hasMinDamage or hasRecentCombat) then
                        eligiblePlayers[memberGuid] = member
                    end
                end
            end
        end
    
    -- Not in party: credit significant damage contributors
    elseif killerDamage < SOLO_KILL_THRESHOLD then
        for attackerId, damage in pairs(damageMap) do
            local damagePercent = damage.total / targetMaxHealth
            if damagePercent >= DAMAGE_THRESHOLD then
                local attacker = Player(attackerId)
                if attacker then
                    eligiblePlayers[attacker:getGuid()] = attacker
                end
            end
        end
    end
    
    -- Give credit to all eligible players
    for _, eligiblePlayer in pairs(eligiblePlayers) do
        giveTaskSystemCredit(eligiblePlayer, targetName, targetId)
    end
    
    return true
end
ev:register()

-- Logout event to clean up recent kills and combat activity
local logoutEv = CreatureEvent("ModernTaskLogout")
function logoutEv.onLogout(player)
    local playerGuid = player:getGuid()
    recentKills[playerGuid] = nil
    recentCombatActivity[playerGuid] = nil
    return true
end
logoutEv:register()

-- Login/Opcode
local login = CreatureEvent("ModernTaskLogin")
function login.onLogin(player)
    player:registerEvent("ModernTaskKill")
    player:registerEvent("ModernTaskLogout")
    player:registerEvent("ModernTaskOpcode")
    return true
end
login:register()

local op = CreatureEvent("ModernTaskOpcode")
function op.onExtendedOpcode(player, opcode, buffer)
    if opcode ~= OPCODE then return end
    local status, data = pcall(json.decode, buffer)
    if not status then return end
    
    if data.action == "refresh" then
        sendData(player)
        return
    end
    
    updateCompletions(player)
    local dailyCount = getStorage(player, config.storage.daily_count)
    local weeklyCount = getStorage(player, config.storage.weekly_count)
    
    -- ========== KITNOF START (id >= 1000) - Must be checked FIRST ==========
    if data.action == "start" and data.id and data.id >= 1000 then
        local taskId = data.id - 1000
        local t = tasks and tasks[taskId]
        local maxTasks = tasksByPlayer or 3
        
        if not t then
            player:sendTextMessage(MESSAGE_STATUS_SMALL, "Task not found.")
            return
        end
        
        -- Check if player has joined Paw & Fur
        if player:getStorageValue(Storage.KillingInTheNameOf.Join) < 0 then
            player:sendTextMessage(MESSAGE_STATUS_SMALL, "You must join Paw & Fur first. Talk to Grizzly Adams.")
            return
        end
        
        local startedTasks = player:getStartedTasks() or {}
        
        if #startedTasks >= maxTasks then
            player:sendTextMessage(MESSAGE_STATUS_SMALL, "You already have " .. maxTasks .. " active Paw & Fur tasks.")
            return
        end
        
        if not player:canStartTask(t.raceName) then
            player:sendTextMessage(MESSAGE_STATUS_SMALL, "You cannot start this task (level/rank requirement not met or already completed).")
            return
        end
        
        -- Start the task
        player:setStorageValue(Storage.KillingInTheNameOf.QuestStorageBase + taskId, 1)
        player:setStorageValue(Storage.KillingInTheNameOf.KillsStorageBase + taskId, 0)
        player:setStorageValue(2501, #player:getStartedTasks())
        player:registerEvent("KillingInTheNameOfKills")
        player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "Task started: " .. (t.name or t.raceName))
        sendData(player)
        return
    
    -- ========== Daily/Weekly START (id < 1000) ==========
    elseif data.action == "start" and data.id and data.id < 1000 then
        local id = data.id
        local isDaily = (id >= 101 and id <= 199)
        local index = isDaily and (id - 100) or (id - 200)
        local pool = isDaily and config.pool.daily or config.pool.weekly
        local t = pool[index]
        
        if not t then return end
        
        local playerLevel = player:getLevel()
        
        -- Validate Level Requirement
        -- The display (addTasksToPayload) already filters tasks by level and rotation,
        -- so the player can only see tasks they're eligible for. We just need to guard
        -- against spoofed/stale requests with a level check.
        if (t.minLevel and playerLevel < t.minLevel) or (t.maxLevel and playerLevel > t.maxLevel) then
             player:sendTextMessage(MESSAGE_STATUS_SMALL, "You do not meet the level requirements for this task.")
             return
        end
        
        local state = getStorage(player, config.storage.state_start + id)
        
        if state == 0 then
            -- Check how many tasks started this period (daily_count / weekly_count)
            local startedCount = isDaily and dailyCount or weeklyCount
            local limit = isDaily and config.limits.daily or config.limits.weekly
            
            if startedCount >= limit then
                local taskType = isDaily and "daily" or "weekly"
                local resetTime = isDaily and "tomorrow" or "next week"
                player:sendTextMessage(MESSAGE_STATUS_SMALL, "You can only start " .. limit .. " " .. taskType .. " tasks. Try again " .. resetTime .. ".")
                return
            end
            
            -- Increment the started count
            if isDaily then
                setStorage(player, config.storage.daily_count, dailyCount + 1)
            else
                setStorage(player, config.storage.weekly_count, weeklyCount + 1)
            end
            
            setStorage(player, config.storage.state_start + id, 1)
            setStorage(player, config.storage.kills_start + id, 0)
            sendData(player)
        end
        
    -- ========== KITNOF CANCEL (id >= 1000) ==========
    elseif data.action == "cancel" and data.id and data.id >= 1000 then
        local taskId = data.id - 1000
        local startedTasks = player:getStartedTasks() or {}
        if isInArray(startedTasks, taskId) then
            player:setStorageValue(Storage.KillingInTheNameOf.QuestStorageBase + taskId, -1)
            player:setStorageValue(Storage.KillingInTheNameOf.KillsStorageBase + taskId, -1)
            player:setStorageValue(2501, #player:getStartedTasks())
            player:sendTextMessage(MESSAGE_STATUS_SMALL, "Paw & Fur task cancelled.")
            sendData(player)
        end

    -- ========== KITNOF CLAIM (id >= 1000) ==========
    elseif data.action == "claim" and data.id and data.id >= 1000 then
        local taskId = data.id - 1000
        local t = tasks and tasks[taskId]
        if not t then return end
        
        local kills = player:getStorageValue(Storage.KillingInTheNameOf.KillsStorageBase + taskId)
        if kills >= t.killsRequired then
            -- Give rewards
            for _, reward in ipairs(t.rewards) do
                local deny = false
                if reward.storage then
                    local rewardStorage = math.max(0, player:getStorageValue(reward.storage[1]))
                    if rewardStorage >= reward.storage[2] then deny = true end
                end
                
                if not deny then
                    local rtype = type(reward.type) == "string" and reward.type:lower() or ""
                    if isInArray({REWARD_MONEY, 'money'}, rtype) then 
                        player:addMoney(reward.value[1])
                    elseif isInArray({REWARD_EXP, 'exp', 'experience'}, rtype) then 
                        player:addExperience(reward.value[1], true)
                    elseif isInArray({REWARD_ACHIEVEMENT, 'achievement', 'ach'}, rtype) then 
                        player:addAchievement(reward.value[1])
                    elseif isInArray({REWARD_POINT, 'points', 'point'}, rtype) then
                        player:setStorageValue(Storage.KillingInTheNameOf.Points, math.max(0, player:getStorageValue(Storage.KillingInTheNameOf.Points)) + reward.value[1])
                    elseif isInArray({REWARD_ITEM, 'item', 'items', 'object'}, rtype) then 
                        player:addItem(reward.value[1], reward.value[2])
                    elseif isInArray({REWARD_STORAGE, 'storage', 'stor'}, rtype) then 
                        player:setStorageValue(reward.value[1], reward.value[2])
                    end
                end
                if reward.storage then 
                    player:setStorageValue(reward.storage[1], reward.storage[2]) 
                end
            end
            
            -- Reset storage
            player:setStorageValue(Storage.KillingInTheNameOf.QuestStorageBase + taskId, t.norepeatable and 2 or 0)
            player:setStorageValue(Storage.KillingInTheNameOf.KillsStorageBase + taskId, -1)
            local repeats = math.max(0, player:getStorageValue(Storage.KillingInTheNameOf.RepeatStorageBase + taskId))
            player:setStorageValue(Storage.KillingInTheNameOf.RepeatStorageBase + taskId, repeats + 1)
            player:setStorageValue(2501, #player:getStartedTasks())
            
            player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "Paw & Fur task completed: " .. (t.name or t.raceName))
            sendData(player)
        end

    -- ========== Daily/Weekly CLAIM (id < 1000) ==========
    elseif data.action == "claim" and data.id and data.id < 1000 then
        local id = data.id
        local isDaily = (id >= 101 and id <= 199)
        local index = isDaily and (id - 100) or (id - 200)
        local pool = isDaily and config.pool.daily or config.pool.weekly
        local t = pool[index]
        
        if t and getStorage(player, config.storage.state_start + id) == 2 then
            -- Check if task was extended (2x rewards)
            local isExtended = getStorage(player, Storage.TaskSystem.ExtendedBase + id) == 1
            local rewardMultiplier = isExtended and config.extend.multiplier or 1
            
            -- Check reward preference (0 = EXP, 1 = Gold)
            local rewardPref = getStorage(player, config.storage.reward_preference)
            local playerLevel = player:getLevel()
            local stageMultiplier = getExpStageMultiplier(playerLevel)
            
            for _, r in ipairs(t.rewards) do
                if r.type == "exp" and rewardPref ~= 1 then
                    -- EXP preference: apply stage multiplier and extend multiplier
                    local expValue = math.floor(r.value * rewardMultiplier * stageMultiplier)
                    player:addExperience(expValue, true)
                elseif r.type == "gold" and rewardPref == 1 then
                    -- Gold preference: use flat gold value, only extend multiplier applies
                    local goldValue = r.value * rewardMultiplier
                    player:addMoney(goldValue)
                    player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "You received " .. goldValue .. " gold!")
                elseif r.type == "points" then setStorage(player, config.storage.points, getStorage(player, config.storage.points) + (r.value * rewardMultiplier))
                elseif r.type == "money" then player:addMoney(r.value * rewardMultiplier)
                elseif r.type == "item" then player:addItem(r.id, r.count * rewardMultiplier) end
            end
            
            -- Track lifetime completions for rank system
            local totalCompleted = getStorage(player, Storage.TaskSystem.TotalTasksCompleted) + 1
            setStorage(player, Storage.TaskSystem.TotalTasksCompleted, totalCompleted)
            if isDaily then
                setStorage(player, Storage.TaskSystem.DailyTasksCompleted, getStorage(player, Storage.TaskSystem.DailyTasksCompleted) + 1)
            else
                setStorage(player, Storage.TaskSystem.WeeklyTasksCompleted, getStorage(player, Storage.TaskSystem.WeeklyTasksCompleted) + 1)
            end
            
            setStorage(player, config.storage.state_start + id, 3)
            -- Reset extended flag
            setStorage(player, Storage.TaskSystem.ExtendedBase + id, 0)
            
            local claimMsg = isExtended and "Extended Task Claimed! (2x Rewards)" or "Task Claimed!"
            player:sendTextMessage(MESSAGE_EVENT_ADVANCE, claimMsg)
            sendData(player)
        end
        
    elseif data.action == "buy" and data.id then
        local item = config.shop[data.id]
        if item then
            local ptr = getStorage(player, config.storage.points)
            if ptr >= item.cost then
                -- Check if onBuy returns false (for exclusive items like upgrades that are already owned)
                local success = true
                if item.onBuy then 
                    local result = item.onBuy(player) 
                    if result == false then success = false end
                end
                
                if success then
                    setStorage(player, config.storage.points, ptr - item.cost)
                    sendData(player)
                end
            else
                player:sendTextMessage(MESSAGE_STATUS_SMALL, "Not enough points.")
            end
        end
    
    -- ========== EXTEND TASK (id < 1000) ==========
    elseif data.action == "extend" and data.id and data.id < 1000 then
        local id = data.id
        local isDaily = (id >= 101 and id <= 199)
        local index = isDaily and (id - 100) or (id - 200)
        local pool = isDaily and config.pool.daily or config.pool.weekly
        local t = pool[index]
        
        if not t then
            player:sendTextMessage(MESSAGE_STATUS_SMALL, "Task not found.")
            return
        end
        
        -- Check if task is active
        local state = getStorage(player, config.storage.state_start + id)
        if state ~= 1 then
            player:sendTextMessage(MESSAGE_STATUS_SMALL, "You can only extend active tasks.")
            return
        end
        
        -- Check if already extended
        if getStorage(player, Storage.TaskSystem.ExtendedBase + id) == 1 then
            player:sendTextMessage(MESSAGE_STATUS_SMALL, "This task is already extended.")
            return
        end
        
        -- Check progress (must be < maxProgress%)
        local kills = getStorage(player, config.storage.kills_start + id)
        local progress = (kills / t.count) * 100
        if progress >= config.extend.maxProgress then
            player:sendTextMessage(MESSAGE_STATUS_SMALL, "You can only extend tasks that are less than " .. config.extend.maxProgress .. "% complete.")
            return
        end
        
        -- Check points
        local ptr = getStorage(player, config.storage.points)
        if ptr < config.extend.cost then
            player:sendTextMessage(MESSAGE_STATUS_SMALL, "Not enough points. Need " .. config.extend.cost .. " points.")
            return
        end
        
        -- Extend the task!
        setStorage(player, config.storage.points, ptr - config.extend.cost)
        setStorage(player, Storage.TaskSystem.ExtendedBase + id, 1)
        
        local newTarget = t.count * config.extend.multiplier
        player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "Task Extended! New target: " .. newTarget .. " kills for 2x rewards!")
        sendData(player)
    
    -- ========== TOGGLE REWARD PREFERENCE ==========
    elseif data.action == "toggleReward" then
        local currentPref = getStorage(player, config.storage.reward_preference)
        local newPref = (currentPref == 1) and 0 or 1
        setStorage(player, config.storage.reward_preference, newPref)
        
        local prefName = (newPref == 1) and "Gold" or "Experience"
        player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "Task rewards set to: " .. prefName)
        sendData(player)
    end
end
op:type("extendedopcode")
op:register()

local cmd = TalkAction("!task")
function cmd.onSay(player, words, type)
    player:sendExtendedOpcode(OPCODE, json.encode({action = "open"}))
    return false
end
cmd:register()

-- ========== DAMAGE BOOST SYSTEM ==========
local TASK_DMG_DEBUG = false -- Set to true for debug messages

local dmgEvent = CreatureEvent("TaskDamageBoost")
function dmgEvent.onHealthChange(creature, attacker, primaryDamage, primaryType, secondaryDamage, secondaryType, origin)
    if not attacker or not attacker:isPlayer() then return primaryDamage, primaryType, secondaryDamage, secondaryType end
    
    -- Check for Slayer Essence Upgrade
    local hasEssence = attacker:getStorageValue(Storage.TaskSystem.SlayerEssence) == 1
    if not hasEssence then return primaryDamage, primaryType, secondaryDamage, secondaryType end
    
    local name = creature:getName():lower()
    local isTaskMonster = creatureToTaskPool[name] ~= nil
    
    if isTaskMonster then
        if primaryDamage < 0 then primaryDamage = math.floor(primaryDamage * 1.2) end
        if secondaryDamage < 0 then secondaryDamage = math.floor(secondaryDamage * 1.2) end
        if TASK_DMG_DEBUG then
            print("[TaskDmgBoost] " .. attacker:getName() .. " -> " .. name .. " | Boosted: " .. primaryDamage)
        end
    end
    
    return primaryDamage, primaryType, secondaryDamage, secondaryType
end
dmgEvent:register()

-- ========== BIGGER AND BADDER - ELITE MONSTER SYSTEM ==========
local ELITE_DEBUG = false -- Set to true for debug messages
local ELITE_SPAWN_CHANCE = 80 -- 1 in 80 chance

-- Elite monster configuration
local eliteConfig = {
    hpMultiplier = {min = 1.8, max = 2.0},      -- 1.8x to 2x HP
    damageMultiplier = {min = 1.5, max = 1.7},  -- 1.5x to 1.7x damage
    speedMultiplier = 1.2,                       -- 1.2x speed
    expMultiplier = 2.0,                         -- 2x XP
    lootRolls = 2,                               -- 2x loot rolls
    
    -- Bonus roll chances (must sum to 100)
    -- Bonus rolls by difficulty tier
    -- Tier 1 (Low - rats, rotworms, etc): Only Rare gems, no legendary/mythic
    -- Tier 2 (Mid - cyclops, dragons, etc): Rare + Epic gems + Reroll Scroll
    -- Tier 3 (High - demons, behemoths, etc): All gems including Legendary + Reroll Scroll
    bonusRollsByDifficulty = {
        [1] = { -- Low difficulty (1-2 point tasks)
            {chance = 50, type = "points"},          -- 50% - Task points (5-8)
            {chance = 45, type = "common_gem"},      -- 45% - Rare Gem
            {chance = 5,  type = "upgraded_item"}    -- 5%  - Pre-upgraded rare item
        },
        [2] = { -- Mid difficulty (3-4 point tasks)
            {chance = 35, type = "points"},          -- 35% - Task points (7-12)
            {chance = 30, type = "common_gem"},      -- 30% - Rare Gem
            {chance = 18, type = "uncommon_gem"},    -- 18% - Epic Gem
            {chance = 12, type = "reroll_scroll"},   -- 12% - Reroll Scroll
            {chance = 5,  type = "upgraded_item"}    -- 5%  - Pre-upgraded epic item
        },
        [3] = { -- High difficulty (5+ point tasks)
            {chance = 30, type = "points"},          -- 30% - Task points (10-15)
            {chance = 20, type = "common_gem"},      -- 20% - Rare Gem
            {chance = 20, type = "uncommon_gem"},    -- 20% - Epic Gem
            {chance = 15, type = "reroll_scroll"},   -- 15% - Reroll Scroll
            {chance = 10, type = "rare_gem"},        -- 10% - Legendary Coin
            {chance = 5,  type = "upgraded_item"}    -- 5%  - Pre-upgraded legendary item
        }
    },
    
    -- Gem item IDs
    gems = {
        common = 18419,    -- Rare Gem
        uncommon = 18414,  -- Epic Gem
        rare = 24115       -- Legendary Coin
    },
    
    -- Reroll Scroll item ID
    rerollScroll = 18413
}

-- Track elite monsters (monster uid -> {owner = player guid, originalName = name})
local eliteMonsters = {}

-- Get monster difficulty tier based on task rewards
local function getMonsterDifficulty(monsterName)
    local name = monsterName:lower()
    local entries = creatureToTaskPool[name]
    if not entries or #entries == 0 then return 1 end
    
    -- Find highest point reward from any task containing this monster
    local maxPoints = 1
    for _, entry in ipairs(entries) do
        if entry.task and entry.task.rewards then
            for _, reward in ipairs(entry.task.rewards) do
                if reward.type == "points" and reward.value > maxPoints then
                    maxPoints = reward.value
                end
            end
        end
    end
    
    -- Tier based on points: 1-2 = low, 3-4 = mid, 5+ = high
    if maxPoints >= 5 then return 3 end
    if maxPoints >= 3 then return 2 end
    return 1
end

-- Calculate bonus points based on monster difficulty
local function getBonusPoints(difficulty)
    if difficulty >= 3 then return math.random(10, 15) end
    if difficulty >= 2 then return math.random(7, 12) end
    return math.random(5, 8)
end

-- Spawn elite version of a monster
local function spawnEliteMonster(position, monsterName, ownerGuid)
    local monster = Game.createMonster(monsterName, position, false, true)
    if not monster then
        if ELITE_DEBUG then print("[Elite] Failed to spawn: " .. monsterName) end
        return nil
    end
    
    -- Calculate random multipliers
    local hpMult = eliteConfig.hpMultiplier.min + math.random() * (eliteConfig.hpMultiplier.max - eliteConfig.hpMultiplier.min)
    local dmgMult = eliteConfig.damageMultiplier.min + math.random() * (eliteConfig.damageMultiplier.max - eliteConfig.damageMultiplier.min)
    
    -- Apply HP boost
    local baseMaxHealth = monster:getMaxHealth()
    local newMaxHealth = math.floor(baseMaxHealth * hpMult)
    monster:setMaxHealth(newMaxHealth)
    monster:setHealth(newMaxHealth)
    
    -- Apply speed boost
    local baseSpeed = monster:getBaseSpeed()
    local newSpeed = math.floor(baseSpeed * eliteConfig.speedMultiplier)
    monster:changeSpeed(newSpeed - baseSpeed)
    
    -- Visual distinction - give elite a skull
    monster:setSkull(SKULL_WHITE)
    
    -- Store elite data
    local uid = monster:getId()
    eliteMonsters[uid] = {
        owner = ownerGuid,
        originalName = monsterName,
        damageMultiplier = dmgMult,
        difficulty = getMonsterDifficulty(monsterName)
    }
    
    -- Register events
    monster:registerEvent("EliteMonsterDeath")
    
    -- Visual indicator - announce elite spawn
    monster:say("ELITE " .. monsterName:upper(), TALKTYPE_MONSTER_SAY)
    position:sendMagicEffect(CONST_ME_HOLYAREA)
    
    if ELITE_DEBUG then
        print("[Elite] Spawned: Elite " .. monsterName .. " | HP: " .. baseMaxHealth .. " -> " .. newMaxHealth .. " | Speed: " .. baseSpeed .. " -> " .. newSpeed)
    end
    
    return monster
end

-- Roll bonus reward for elite kill
local function rollEliteBonus(player, monsterName, corpse)
    local roll = math.random(1, 100)
    local cumulative = 0
    local difficulty = getMonsterDifficulty(monsterName)
    
    -- Get the appropriate bonus table for this difficulty
    local bonusTable = eliteConfig.bonusRollsByDifficulty[difficulty] or eliteConfig.bonusRollsByDifficulty[1]
    
    if ELITE_DEBUG then
        print("[Elite] Rolling bonus for difficulty " .. difficulty .. ", roll: " .. roll)
    end
    
    for _, bonus in ipairs(bonusTable) do
        cumulative = cumulative + bonus.chance
        if roll <= cumulative then
            if bonus.type == "points" then
                local points = getBonusPoints(difficulty)
                local current = math.max(0, player:getStorageValue(Storage.TaskSystem.Points))
                player:setStorageValue(Storage.TaskSystem.Points, current + points)
                player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "Elite Bonus: +" .. points .. " Task Points!")
                return
                
            elseif bonus.type == "common_gem" then
                player:addItem(eliteConfig.gems.common, 1)
                player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "Elite Bonus: Rare Gem!")
                return
                
            elseif bonus.type == "uncommon_gem" then
                player:addItem(eliteConfig.gems.uncommon, 1)
                player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "Elite Bonus: Epic Gem!")
                return
                
            elseif bonus.type == "rare_gem" then
                player:addItem(eliteConfig.gems.rare, 1)
                player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "Elite Bonus: Legendary Coin!")
                return
                
            elseif bonus.type == "reroll_scroll" then
                player:addItem(eliteConfig.rerollScroll, 1)
                player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "Elite Bonus: Reroll Scroll!")
                return
                
            elseif bonus.type == "upgraded_item" then
                -- Try to upgrade a random item from the corpse
                -- Tier of upgrade depends on difficulty
                local forcedTier = "rare"
                if difficulty >= 3 then
                    local tierRoll = math.random(1, 100)
                    -- High tier: 2% Mythic, 10% Legendary, 38% Epic, 50% Rare
                    if tierRoll <= 2 then
                        forcedTier = "mythic"
                    elseif tierRoll <= 12 then
                        forcedTier = "legendary"
                    elseif tierRoll <= 50 then
                        forcedTier = "epic"
                    else
                        forcedTier = "rare"
                    end
                elseif difficulty >= 2 then
                    local tierRoll = math.random(1, 100)
                    forcedTier = tierRoll <= 40 and "epic" or "rare"
                end
                
                if corpse and corpse:isContainer() then
                    local items = {}
                    for i = 0, corpse:getSize() - 1 do
                        local item = corpse:getItem(i)
                        if item then
                            local itemType = ItemType(item:getId())
                            if not itemType:isStackable() and (itemType:getWeaponType() > 0 or itemType:getArmor() > 0) then
                                table.insert(items, item)
                            end
                        end
                    end
                    
                    if #items > 0 then
                        local targetItem = items[math.random(1, #items)]
                        if rollRarity then
                            rollRarity(targetItem, forcedTier)
                            player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "Elite Bonus: Pre-upgraded " .. forcedTier:gsub("^%l", string.upper) .. " item!")
                            return
                        end
                    end
                end
                -- Fallback to gem based on difficulty if no upgradeable item
                if difficulty >= 2 then
                    player:addItem(eliteConfig.gems.uncommon, 1)
                    player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "Elite Bonus: Epic Gem!")
                else
                    player:addItem(eliteConfig.gems.common, 1)
                    player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "Elite Bonus: Rare Gem!")
                end
                return
            end
        end
    end
end

-- Helper: Get eligible party members for elite rewards (reuses task system anti-leech logic)
local function getEliteEligiblePlayers(killer, mostDamageKiller, creature, ownerGuid)
    -- Find the primary player
    local player = killer
    if not player or not player:isPlayer() then
        player = mostDamageKiller
    end
    if not player or not player:isPlayer() then
        player = Player(ownerGuid)
    end
    if not player or not player:isPlayer() then return {} end
    
    local eligible = {}
    eligible[player:getGuid()] = player
    
    local party = player:getParty()
    if party then
        local members = party:getMembers()
        members[#members + 1] = party:getLeader()
        
        local creaturePos = creature:getPosition()
        local damageMap = creature:getDamageMap()
        local maxHealth = creature:getMaxHealth()
        
        for _, member in ipairs(members) do
            if member and member:getGuid() ~= player:getGuid() then
                if member:getPosition():getDistance(creaturePos) <= PARTY_SHARE_RANGE then
                    local withinLevelRange = isWithinPartyLevelRange(member, party)
                    
                    local memberRuntimeId = member:getId()
                    local memberDamage = damageMap[memberRuntimeId]
                    local damagePercent = memberDamage and (memberDamage.total / maxHealth) or 0
                    local hasMinDamage = damagePercent >= PARTY_MIN_DAMAGE
                    local hasRecentCombat = hasRecentCombatActivity(member:getGuid())
                    
                    if withinLevelRange and (hasMinDamage or hasRecentCombat) then
                        eligible[member:getGuid()] = member
                    end
                end
            end
        end
    end
    
    return eligible
end

-- Elite monster death handler — shares rewards with eligible party members
local eliteDeathEvent = CreatureEvent("EliteMonsterDeath")
function eliteDeathEvent.onDeath(creature, corpse, killer, mostDamageKiller, lastHitUnjustified, mostDamageUnjustified)
    local uid = creature:getId()
    local eliteData = eliteMonsters[uid]
    
    if not eliteData then return true end
    
    -- Get all eligible players (killer + party with anti-leech checks)
    local eligiblePlayers = getEliteEligiblePlayers(killer, mostDamageKiller, creature, eliteData.owner)
    
    if next(eligiblePlayers) then
        -- Extra loot roll — done once into the shared corpse
        if corpse and corpse:isContainer() then
            local monsterType = creature:getType()
            if monsterType then
                local loot = monsterType:getLoot()
                local itemsBefore = corpse:getItemHoldingCount()
                for i = 1, #loot do
                    corpse:createLootItem(loot[i])
                end
                local itemsAfter = corpse:getItemHoldingCount()
                if ELITE_DEBUG then
                    print("[Elite] Extra loot roll: " .. #loot .. " loot entries processed, corpse items: " .. itemsBefore .. " -> " .. itemsAfter)
                end
            end
        else
            if ELITE_DEBUG then
                print("[Elite] WARNING: No corpse found for extra loot!")
            end
        end
        
        -- Distribute bonus XP to all eligible players, bonus roll to ONE random player
        local monsterType = creature:getType()
        local baseExp = monsterType and monsterType:getExperience() or 0
        local playerList = {}
        for _, player in pairs(eligiblePlayers) do
            playerList[#playerList + 1] = player
        end
        local playerCount = #playerList
        
        for _, player in ipairs(playerList) do
            -- Bonus XP — split evenly among eligible players (same as normal party exp sharing)
            if baseExp > 0 then
                local sharedExp = math.floor(baseExp / playerCount)
                player:addExperience(sharedExp, true)
                if ELITE_DEBUG then
                    print("[Elite] Bonus XP: " .. sharedExp .. " to " .. player:getName() .. " (split " .. playerCount .. " ways)")
                end
            end
        end
        
        -- Single bonus roll — one random eligible player who has Bigger and Badder wins
        local upgradeHolders = {}
        for _, player in ipairs(playerList) do
            if player:getStorageValue(Storage.TaskSystem.BiggerAndBadder) == 1 then
                upgradeHolders[#upgradeHolders + 1] = player
            end
        end
        if #upgradeHolders > 0 then
            local bonusWinner = upgradeHolders[math.random(1, #upgradeHolders)]
            rollEliteBonus(bonusWinner, eliteData.originalName, corpse)
        end
        
        -- Announce
        local pos = creature:getPosition()
        if pos then
            pos:sendMagicEffect(CONST_ME_FIREWORK_YELLOW)
        end
    end
    
    -- Cleanup
    eliteMonsters[uid] = nil
    return true
end
eliteDeathEvent:register()

-- Elite monster damage boost handler
local eliteDmgEvent = CreatureEvent("EliteMonsterDamage")
function eliteDmgEvent.onHealthChange(creature, attacker, primaryDamage, primaryType, secondaryDamage, secondaryType, origin)
    -- This fires when the player (creature) takes damage from attacker
    -- We boost damage if attacker is an elite monster
    if not attacker then return primaryDamage, primaryType, secondaryDamage, secondaryType end
    
    local uid = attacker:getId()
    local eliteData = eliteMonsters[uid]
    
    if eliteData and eliteData.damageMultiplier then
        local oldPrimary = primaryDamage
        if primaryDamage < 0 then
            primaryDamage = math.floor(primaryDamage * eliteData.damageMultiplier)
        end
        if secondaryDamage < 0 then
            secondaryDamage = math.floor(secondaryDamage * eliteData.damageMultiplier)
        end
        if ELITE_DEBUG then
            print("[Elite] Damage boost: " .. oldPrimary .. " -> " .. primaryDamage .. " (x" .. string.format("%.2f", eliteData.damageMultiplier) .. ")")
        end
    end
    
    return primaryDamage, primaryType, secondaryDamage, secondaryType
end
eliteDmgEvent:register()

-- Track recent kills to prevent double spawns
local recentEliteSpawns = {}

-- Check if player has an active task for this monster
local function hasActiveTaskFor(player, monsterName)
    local entries = creatureToTaskPool[monsterName:lower()]
    if not entries then return false end
    
    for _, entry in ipairs(entries) do
        local state = getStorage(player, config.storage.state_start + entry.id)
        if state == 1 then -- Active task
            return true
        end
    end
    return false
end

-- Hook into task monster kills to potentially spawn elite
-- Checks ALL party members for the upgrade, not just the last-hitter
local eliteSpawnEvent = CreatureEvent("EliteSpawnCheck")
function eliteSpawnEvent.onKill(creature, target)
    if not creature or not creature:isPlayer() then return true end
    if not target or target:isPlayer() then return true end
    
    local targetId = target:getId()
    local targetName = target:getName():lower()
    
    -- Prevent double spawn from same kill
    if recentEliteSpawns[targetId] then
        if ELITE_DEBUG then print("[Elite] Skipped: recentEliteSpawns guard for " .. targetName .. " (killer: " .. creature:getName() .. ")") end
        return true
    end
    
    -- Check if it's a task monster at all
    if not creatureToTaskPool[targetName] then return true end
    
    -- Don't spawn elite from elite
    if eliteMonsters[targetId] then return true end
    
    if ELITE_DEBUG then print("[Elite] Kill detected: " .. targetName .. " by " .. creature:getName()) end
    
    -- Gather all eligible players: killer + party members in range who have the upgrade + active task
    local triggerPlayer = nil
    local targetPos = target:getPosition()
    
    -- Build candidate list: killer first, then party members
    local candidates = {creature}
    local party = creature:getParty()
    if party then
        local members = party:getMembers()
        members[#members + 1] = party:getLeader()
        for _, member in ipairs(members) do
            if member and member:getGuid() ~= creature:getGuid() then
                if member:getPosition():getDistance(targetPos) <= PARTY_SHARE_RANGE then
                    candidates[#candidates + 1] = member
                end
            end
        end
        if ELITE_DEBUG then print("[Elite] Party found: " .. #candidates .. " candidates (including killer)") end
    else
        if ELITE_DEBUG then print("[Elite] Solo player, no party") end
    end
    
    -- Check if ANY candidate has the upgrade + active task
    for _, candidate in ipairs(candidates) do
        local hasUpgrade = candidate:getStorageValue(Storage.TaskSystem.BiggerAndBadder) == 1
        local hasTask = hasActiveTaskFor(candidate, targetName)
        if ELITE_DEBUG then print("[Elite] Candidate: " .. candidate:getName() .. " | upgrade=" .. tostring(hasUpgrade) .. " | activeTask=" .. tostring(hasTask)) end
        if hasUpgrade and hasTask then
            triggerPlayer = candidate
            break
        end
    end
    
    if not triggerPlayer then
        if ELITE_DEBUG then print("[Elite] No eligible candidate found, skipping") end
        return true
    end
    
    -- Roll for elite spawn (1 in ELITE_SPAWN_CHANCE) — single roll, no stacking
    local roll = math.random(1, ELITE_SPAWN_CHANCE)
    if ELITE_DEBUG then print("[Elite] Roll: " .. roll .. "/" .. ELITE_SPAWN_CHANCE .. " (need 1) | trigger: " .. triggerPlayer:getName()) end
    if roll ~= 1 then return true end
    
    -- Mark as spawning to prevent duplicates
    recentEliteSpawns[targetId] = true
    
    -- Spawn elite!
    local position = target:getPosition()
    local monsterName = target:getName()
    
    -- Small delay to let corpse appear first
    addEvent(function(pos, name, guid, tid)
        -- Cleanup tracking
        recentEliteSpawns[tid] = nil
        
        local player = Player(guid)
        if player then
            local elite = spawnEliteMonster(pos, name, guid)
            if elite then
                player:sendTextMessage(MESSAGE_EVENT_ADVANCE, "An Elite " .. name .. " has appeared!")
            end
        end
    end, 500, position, monsterName, triggerPlayer:getGuid(), targetId)
    
    return true
end
eliteSpawnEvent:register()
