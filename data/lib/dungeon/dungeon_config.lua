--[[
    Dungeon System — Configuration
    All dungeon definitions are data-driven. No hardcoded logic.
    Positions use dx/dy offsets from slot entrance for portability across slots.
]]

DungeonConfig = {}

DungeonConfig[1] = {
    id = 1,
    name = "Crypt of Shadows",
    description = "An ancient crypt overrun by undead horrors. Defeat the guardians and face Lord Shadowbone.",
    minLevel = 80,
    maxPlayers = 5,
    minPlayers = 2,
    timeLimit = 2400, -- 40 minutes
    difficulty = "Normal",

    -- Map slot definitions (pre-built in RME at these coordinates)
    slots = {
        [1] = {
            entrance  = Position(60000, 60000, 7),
            exit      = Position(60005, 60000, 7),
            graveyard = Position(60002, 60002, 7),
            bossRoom  = Position(60050, 60050, 7),
            fromPos   = Position(59990, 59990, 7),
            toPos     = Position(60100, 60100, 7),
        },
        [2] = {
            entrance  = Position(60200, 60000, 7),
            exit      = Position(60205, 60000, 7),
            graveyard = Position(60202, 60002, 7),
            bossRoom  = Position(60250, 60050, 7),
            fromPos   = Position(60190, 59990, 7),
            toPos     = Position(60300, 60100, 7),
        },
        [3] = {
            entrance  = Position(60400, 60000, 7),
            exit      = Position(60405, 60000, 7),
            graveyard = Position(60402, 60002, 7),
            bossRoom  = Position(60450, 60050, 7),
            fromPos   = Position(60390, 59990, 7),
            toPos     = Position(60500, 60100, 7),
        },
        [4] = {
            entrance  = Position(60600, 60000, 7),
            exit      = Position(60605, 60000, 7),
            graveyard = Position(60602, 60002, 7),
            bossRoom  = Position(60650, 60050, 7),
            fromPos   = Position(60590, 59990, 7),
            toPos     = Position(60700, 60100, 7),
        },
    },

    -- Encounter sequence (linear progression)
    encounters = {
        [1] = {
            type = "trash",
            name = "Crypt Guardians",
            spawns = {
                {monster = "crypt shambler", pos = {dx = 10, dy = 5}, count = 3},
                {monster = "bonelord", pos = {dx = 12, dy = 8}, count = 2},
            },
            triggerType = "proximity",
            triggerPos = {dx = 8, dy = 5},
            triggerRadius = 5,
        },
        [2] = {
            type = "trash",
            name = "Skeleton Patrol",
            spawns = {
                {monster = "skeleton warrior", pos = {dx = 20, dy = 10}, count = 4},
                {monster = "ghost", pos = {dx = 22, dy = 12}, count = 2},
            },
            triggerType = "kill_previous",
        },
        [3] = {
            type = "boss",
            name = "Lord Shadowbone",
            spawns = {
                {monster = "undead dragon", pos = {dx = 50, dy = 50}, count = 1},
            },
            triggerType = "kill_previous",
            phases = {
                [1] = {hpPercent = 100, mechanics = {}},
                [2] = {hpPercent = 50, mechanics = {"summon_adds"},
                    adds = {
                        {monster = "skeleton warrior", count = 4},
                    }},
                [3] = {hpPercent = 25, mechanics = {"enrage"},
                    enrage = {damageMultiplier = 1.5, speedBoost = 30}},
            },
            loot = {
                {itemId = 2400, chance = 15000, name = "Magic Plate Armor"},
                {itemId = 2472, chance = 20000, name = "Magic Longsword"},
                {itemId = 2160, count = {5, 15}, chance = 100000, name = "Crystal Coin"},
            },
        },
    },

    completionRewards = {
        experience = 500000,
        money = 50000,
    },
}

DungeonConfig[2] = {
    id = 2,
    name = "Infernal Depths",
    description = "The burning pits of demon territory. Two bosses guard the infernal treasure.",
    minLevel = 150,
    maxPlayers = 5,
    minPlayers = 2,
    timeLimit = 3000, -- 50 minutes
    difficulty = "Heroic",

    slots = {
        [1] = {
            entrance  = Position(61000, 60000, 7),
            exit      = Position(61005, 60000, 7),
            graveyard = Position(61002, 60002, 7),
            bossRoom  = Position(61050, 60050, 7),
            fromPos   = Position(60990, 59990, 7),
            toPos     = Position(61100, 60100, 7),
        },
        [2] = {
            entrance  = Position(61200, 60000, 7),
            exit      = Position(61205, 60000, 7),
            graveyard = Position(61202, 60002, 7),
            bossRoom  = Position(61250, 60050, 7),
            fromPos   = Position(61190, 59990, 7),
            toPos     = Position(61300, 60100, 7),
        },
    },

    encounters = {
        [1] = {
            type = "trash",
            name = "Demon Vanguard",
            spawns = {
                {monster = "fire devil", pos = {dx = 10, dy = 5}, count = 4},
                {monster = "fire elemental", pos = {dx = 14, dy = 8}, count = 3},
            },
            triggerType = "proximity",
            triggerPos = {dx = 8, dy = 5},
            triggerRadius = 5,
        },
        [2] = {
            type = "trash",
            name = "Hellfire Patrol",
            spawns = {
                {monster = "hellfire fighter", pos = {dx = 25, dy = 15}, count = 3},
                {monster = "diabolic imp", pos = {dx = 28, dy = 18}, count = 4},
            },
            triggerType = "kill_previous",
        },
        [3] = {
            type = "boss",
            name = "Infernal Guardian",
            spawns = {
                {monster = "demon", pos = {dx = 40, dy = 35}, count = 1},
            },
            triggerType = "kill_previous",
            phases = {
                [1] = {hpPercent = 100, mechanics = {}},
                [2] = {hpPercent = 40, mechanics = {"summon_adds"},
                    adds = {{monster = "fire elemental", count = 3}}},
            },
            loot = {
                {itemId = 2494, chance = 12000, name = "Demon Helmet"},
                {itemId = 2160, count = {8, 20}, chance = 100000, name = "Crystal Coin"},
            },
        },
        [4] = {
            type = "trash",
            name = "Inner Sanctum",
            spawns = {
                {monster = "destroyer", pos = {dx = 55, dy = 40}, count = 3},
                {monster = "plaguesmith", pos = {dx = 58, dy = 42}, count = 2},
            },
            triggerType = "kill_previous",
        },
        [5] = {
            type = "boss",
            name = "Archfiend",
            spawns = {
                {monster = "grim reaper", pos = {dx = 70, dy = 55}, count = 1},
            },
            triggerType = "kill_previous",
            phases = {
                [1] = {hpPercent = 100, mechanics = {}},
                [2] = {hpPercent = 60, mechanics = {"area_denial"}},
                [3] = {hpPercent = 30, mechanics = {"enrage", "summon_adds"},
                    enrage = {damageMultiplier = 2.0, speedBoost = 50},
                    adds = {{monster = "destroyer", count = 2}}},
            },
            loot = {
                {itemId = 2400, chance = 10000, name = "Magic Plate Armor"},
                {itemId = 2472, chance = 15000, name = "Magic Longsword"},
                {itemId = 2160, count = {15, 30}, chance = 100000, name = "Crystal Coin"},
            },
        },
    },

    completionRewards = {
        experience = 1200000,
        money = 120000,
    },
}
