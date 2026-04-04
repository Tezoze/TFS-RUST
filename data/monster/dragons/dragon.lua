local mType = Game.createMonsterType("Dragon")
local monster = {}

monster.description = "a dragon"
monster.experience = 700
monster.outfit = {
	lookType = 34,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 5973
monster.health = 1000
monster.maxHealth = 1000
monster.race = "blood"
monster.speed = 172
monster.manaCost = 0
monster.maxSummons = 0

monster.changeTarget = {
	interval = 4000,
	chance = 10
}

monster.flags = {
	summonable = false,
	attackable = true,
	hostile = true,
	illusionable = true,
	convinceable = false,
	pushable = false,
	canPushItems = true,
	canPushCreatures = true,
	targetDistance = 1,
	staticAttackChance = 80,
	runHealth = 300,
	canWalkOnEnergy = true,
	canWalkOnFire = true,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "GROOAAARRR", yell = true},
	{text = "FCHHHHH", yell = true},
}

monster.loot = {
	{id = 2145, chance = 380}, -- small diamond
	{id = 2148, chance = 47500, maxCount = 70}, -- gold coin
	{id = 2148, chance = 37500, maxCount = 45}, -- gold coin
	{id = 2177, chance = 120}, -- life crystal
	{id = 2187, chance = 1005}, -- wand of inferno
	{id = 2387, chance = 960}, -- double axe
	{id = 2397, chance = 4000}, -- longsword
	{id = 2409, chance = 420}, -- serpent sword
	{id = 2413, chance = 1950}, -- broadsword
	{id = 2434, chance = 560}, -- dragon hammer
	{id = 2455, chance = 10000}, -- crossbow
	{id = 2457, chance = 3000}, -- steel helmet
	{id = 2509, chance = 15000}, -- steel shield
	{id = 2516, chance = 320}, -- dragon shield
	{id = 2546, chance = 8060, maxCount = 10}, -- burst arrow
	{id = 2647, chance = 2000}, -- plate legs
	{id = 2672, chance = 65500, maxCount = 3}, -- dragon ham
	{id = 5877, chance = 1005}, -- green dragon leather
	{id = 5920, chance = 1000}, -- green dragon scale
	{id = 7430, chance = 110}, -- dragonbone staff
	{id = 7588, chance = 1000}, -- strong health potion
	{id = 12413, chance = 9740}, -- dragon's tail
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -120, target = false},
	{name = "combat", interval = 2000, chance = 15, minDamage = -60, maxDamage = -140, range = 7, radius = 4, shootEffect = CONST_ANI_FIRE, effect = CONST_ME_FIREAREA, target = true, type = COMBAT_FIREDAMAGE},
	{name = "combat", interval = 2000, chance = 10, minDamage = -100, maxDamage = -170, effect = CONST_ME_FIREAREA, target = false, length = 8, spread = 3, type = COMBAT_FIREDAMAGE},
}

monster.defenses = {
	defense = 30,
	armor = 25,
	{name = "combat", interval = 2000, chance = 15, minDamage = 40, maxDamage = 70, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
}

monster.elements = {
	{type = COMBAT_EARTHDAMAGE, percent = 80},
	{type = COMBAT_ENERGYDAMAGE, percent = 20},
	{type = COMBAT_ICEDAMAGE, percent = -10},
}

monster.immunities = {
	{type = "paralyze", combat = false, condition = true},
	{type = "invisible", combat = false, condition = true},
	{type = "fire", combat = true, condition = true},
}


mType:register(monster)