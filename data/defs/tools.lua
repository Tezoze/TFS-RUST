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
		[3696] = 3695,
		[3702] = 3701,
	},
	pickGrounds = {354, 355}, -- pick usable ground
	sandIds = {231}, -- desert sand for shovel (scarab coins, scarab spawn)
	holes = {593, 606, 608}, -- holes opened by shovel
	holeId = { -- usable rope holes (for roping creatures/items from below)
		294, 369, 370, 385, 394, 411, 412, 413, 432, 433, 434, 435, 476, 594, 595, 607,
		609, 610, 615, 1156, 482, 483, 1067, 1080, 4824, 4826,
	},
	ropeSpots = {386, 421},
	-- Single-id transforms / spawns. Rust looks up by name â do not hardcode these.
	ids = {
		rushWood = 2130,
		pumpkin = 3594,
		pumpkinhead = 2977,
		pickHoleOpen = 394,
		sandHoleOpen = 615,
		wheatMature = 3653,
		wheatGrowing = 3652,
		wheatCut = 3651,
		wheatBunch = 3605,
		scarabCoin = 3042,
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
		4240, 4241, 4242, 4243, 4246, 4247, 4248,
	},
	-- Native `player_move_policy.rs` â TVP moveitem.lua rules (no hardcoded ids in Rust).
	moveItemPolicy = {
		questObjectAidMin = 1000,
		questObjectAidMax = 2000,
		preMoveTransforms = {
			[2927] = 2912, -- permanent lit candelabrum â expiring
		},
		postMoveTransforms = {
			[3482] = 3481, -- open trap â closed
		},
		postMoveEffectId = 3, -- CONST_ME_POFF after postMoveTransforms
	},
	-- This array contains all destroyable field items
	Fields = {
		2118, 2119, 2120, 2121, 2122, 2123, 2124, 2125,
		2126, 2127, 2131, 2132, 2133, 2134, 2135, 2136,
	},
}
