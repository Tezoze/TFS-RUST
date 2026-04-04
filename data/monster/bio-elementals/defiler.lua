local mType = Game.createMonsterType("Defiler")
local monster = {}

monster.description = "a defiler"
monster.experience = 3700
monster.outfit = {
	lookType = 238,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6532
monster.health = 3650
monster.maxHealth = 3650
monster.race = "venom"
monster.speed = 160
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
	runHealth = 85,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Blubb", yell = false},
	{text = "Blubb Blubb", yell = false},
}

monster.loot = {
	{id = 2145, chance = 2439, maxCount = 2}, -- small diamond
	{id = 2147, chance = 3000, maxCount = 2}, -- small ruby
	{id = 2148, chance = 100000, maxCount = 100}, -- gold coin
	{id = 2148, chance = 100000, maxCount = 100}, -- gold coin
	{id = 2148, chance = 100000, maxCount = 72}, -- gold coin
	{id = 2149, chance = 5366, maxCount = 3}, -- small emerald
	{id = 2151, chance = 5710}, -- talon
	{id = 2152, chance = 95000, maxCount = 6}, -- platinum coin
	{id = 2154, chance = 1219}, -- yellow gem
	{id = 2155, chance = 613}, -- green gem
	{id = 2156, chance = 1538}, -- red gem
	{id = 2158, chance = 300}, -- blue gem
	{id = 5944, chance = 20000}, -- soul orb
	{id = 6300, chance = 3030}, -- death ring
	{id = 6500, chance = 20320}, -- demonic essence
	{id = 9967, chance = 14210}, -- glob of acid slime
	{id = 9968, chance = 12000}, -- glob of tar
}

monster.attacks = {
	{name = "melee", minDamage = 0, maxDamage = -240, interval = 2000, target = false},
	{name = "combat", type = COMBAT_EARTHDAMAGE, minDamage = -160, maxDamage = -270, interval = 2000, chance = 20, range = 7, target = true, shootEffect = CONST_ANI_POISON},
	{name = "condition", type = CONDITION_POISON, interval = 2000, chance = 15, tick = 4000, minDamage = -400, maxDamage = -640, range = 7, radius = 7, effect = CONST_ME_HITBYPOISON, target = false},
	{name = "combat", type = COMBAT_EARTHDAMAGE, minDamage = -120, maxDamage = -170, interval = 2000, chance = 20, radius = 3, target = false, effect = CONST_ME_POISON},
	{name = "condition", type = CONDITION_POISON, interval = 2000, chance = 10, tick = 4000, minDamage = -500, maxDamage = -1000, length = 8, spread = 3, effect = CONST_ME_SMALLPLANTS, target = false},
	{name = "speed", interval = 2000, chance = 15, length = 8, spread = 3, target = false, effect = CONST_ME_SMALLCLOUDS, speed = -700, duration = 15000},
}

monster.defenses = {
	defense = 20,
	armor = 60,
	{name = "combat", interval = 2000, chance = 10, minDamage = 280, maxDamage = 350, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
}

monster.elements = {
	{type = COMBAT_ENERGYDAMAGE, percent = 10},
	{type = COMBAT_ICEDAMAGE, percent = 20},
	{type = COMBAT_FIREDAMAGE, percent = -25},
}

monster.immunities = {
	{type = "earth", combat = true, condition = true},
	{type = "paralyze", combat = false, condition = true},
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)