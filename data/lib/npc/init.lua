-- NPC Builder Framework Loader
-- Loads all modules in dependency order.
-- This file is loaded from data/lib/lib.lua after the Jiddo framework.

-- Foundation modules (no dependencies on each other)
dofile('data/lib/npc/instance_state.lua')
dofile('data/lib/npc/keyword_matcher.lua')
dofile('data/lib/npc/event_dispatcher.lua')

-- Base builder (depends on InstanceState, KeywordMatcher, EventDispatcher)
dofile('data/lib/npc/npc_builder.lua')

-- Specialized builders (depend on NpcBuilder)
dofile('data/lib/npc/shop_builder.lua')
dofile('data/lib/npc/travel_builder.lua')
dofile('data/lib/npc/quest_builder.lua')
dofile('data/lib/npc/dialogue_builder.lua')

-- Cleanup (depends on InstanceState)
dofile('data/lib/npc/cleanup.lua')

print("[NpcBuilder] Framework loaded successfully")
