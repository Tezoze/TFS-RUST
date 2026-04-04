-- Core API functions implemented in Lua
dofile('data/lib/core/core.lua')

-- Shared rarity utilities (used by rarity_scroll and reroll_scroll)
dofile('data/lib/rarity_utils.lua')

-- JSON library (must be loaded early for scripts that use json.encode/decode)
json = dofile('data/lib/json.lua')

-- Quests
dofile('data/lib/quests/killing_in_the_name_of.lua')
dofile('data/lib/quests/demon_oak.lua')
dofile('data/lib/quests/svargrond_arena.lua')

-- Dungeon System
dofile('data/lib/dungeon/dungeon_config.lua')
dofile('data/lib/dungeon/dungeon_manager.lua')
dofile('data/lib/dungeon/dungeon_encounter.lua')
dofile('data/lib/dungeon/dungeon_loot.lua')

-- Compatibility library for our old Lua API
dofile('data/lib/compat/compat.lua')

-- TFS NPC System
dofile('data/npc/lib/npc.lua')
dofile('data/npc/lib/npcsystem/npcsystem.lua')
dofile('data/npc/lib/npcsystem/npchandler.lua')
dofile('data/npc/lib/npcsystem/keywordhandler.lua')
dofile('data/npc/lib/npcsystem/modules.lua')

-- Miscellaneous helper functions
dofile('data/lib/miscellaneous/miscellaneous.lua')

-- NPC Builder framework (modern declarative NPC system)
dofile('data/lib/npc/init.lua')

-- Debugging helper function for Lua developers
dofile('data/lib/debugging/dump.lua')
dofile('data/lib/debugging/lua_version.lua')
