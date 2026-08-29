-- Tool / destroy / field item ids. Loaded at startup by `tool_use.rs`.
-- Pack surface: former `data/global.lua` tables. Spell scripts still receive
-- these as Lua globals via `inject_door_tables_from_global`.

return {
	schema = 1,
	actionIds = {
		puzzleSwitch = 4000, -- a switch that cannot be moved back and will decay into its original ID (puzzle lever)
		sandstoneWall = 4001, -- a sandstone wall that is walkable
		sandHole = 4002, -- hidden sand hole
		pickHole = 4003, -- hidden mud hole
		destroyableStone = 4004, -- stone that is destroyable with a pick on the map
		blockingTile = 4005, -- does not allow any creature to walk through it
	},
	jungleGrass = { -- grass destroyable by machete
		[2782] = 2781,
		[3985] = 3984,
	},
	pickGrounds = {354, 355}, -- pick usable ground
	sandIds = {231}, -- desert sand for shovel (scarab coins, scarab spawn)
	holes = {468, 481, 483}, -- holes opened by shovel
	holeId = { -- usable rope holes (for roping creatures/items from below)
		294, 369, 370, 383, 392, 408, 409, 410, 427, 428, 429, 430, 462, 469, 470, 482,
		484, 485, 489, 924, 3135, 3136, 3311, 3324, 4835, 4837,
	},
	ropeSpots = {384, 418},
	-- Single-id transforms / spawns. Rust looks up by name — do not hardcode these.
	ids = {
		rushWood = 1499,
		pumpkin = 2683,
		pumpkinhead = 2096,
		pickHoleOpen = 392,
		sandHoleOpen = 489,
		wheatMature = 2739,
		wheatGrowing = 2738,
		wheatCut = 2737,
		wheatBunch = 2694,
		scarabCoin = 2159,
	},
	scarab = {
		monster = "Scarab",
		timerSecs = 4000,
		spawnChance = 95,
	},
	chances = {
		sandHole = 20,
	},
	-- All corpses (human corpses), used mostly for desintegrate rune
	corpseIds = {
		3058, 3059, 3060, 3061, 3064, 3065, 3066,
	},
	-- Native `player_move_policy.rs` — TVP moveitem.lua rules (no hardcoded ids in Rust).
	moveItemPolicy = {
		questObjectAidMin = 1000,
		questObjectAidMax = 2000,
		preMoveTransforms = {
			[2057] = 2042, -- permanent lit candelabrum → expiring
		},
		postMoveTransforms = {
			[2579] = 2578, -- open trap → closed
		},
		postMoveEffectId = 3, -- CONST_ME_POFF after postMoveTransforms
	},
	-- This array contains all destroyable field items
	Fields = {
		1487, 1488, 1489, 1490, 1491, 1492, 1493, 1494,
		1495, 1496, 1500, 1501, 1502, 1503, 1504, 1505,
	},
}
