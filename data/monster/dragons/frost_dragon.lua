local mType = Game.createMonsterType("Frost Dragon")
local monster = {}

monster.description = "a frost dragon"
monster.experience = 2100
monster.outfit = {
	lookType = 248,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 7091
monster.health = 1800
monster.maxHealth = 1800
monster.race = "undead"
monster.speed = 212
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
	staticAttackChance = 70,
	runHealth = 250,
	canWalkOnEnergy = false,
	canWalkOnFire = true,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "YOU WILL FREEZE!", yell = true},
	{text = "ZCHHHHH!", yell = true},
	{text = "I am so cool.", yell = false},
	{text = "Chill out!", yell = false},
}

monster.loot = {
	{id = 1976, chance = 8500}, -- book
	{id = 2033, chance = 3000}, -- golden mug
	{id = 2146, chance = 5200}, -- small sapphire
	{id = 2148, chance = 33000, maxCount = 100}, -- gold coin
	{id = 2148, chance = 33000, maxCount = 100}, -- gold coin
	{id = 2148, chance = 33000, maxCount = 42}, -- gold coin
	{id = 2167, chance = 5000}, -- energy ring
	{id = 2177, chance = 520}, -- life crystal
	{id = 2396, chance = 350}, -- ice rapier
	{id = 2479, chance = 450}, -- strange helmet
	{id = 2492, chance = 80}, -- dragon scale mail
	{id = 2498, chance = 210}, -- royal helmet
	{id = 2528, chance = 340}, -- tower shield
	{id = 2547, chance = 6000, maxCount = 6}, -- power bolt
	{id = 2672, chance = 80370, maxCount = 5}, -- dragon ham
	{id = 2796, chance = 12000}, -- green mushroom
	{id = 7290, chance = 5500}, -- shard
	{id = 7402, chance = 120}, -- dragon slayer
	{id = 7441, chance = 4000}, -- ice cube
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -225, target = false},
	{name = "combat", interval = 2000, chance = 10, minDamage = -175, maxDamage = -380, effect = CONST_ME_POFF, target = false, length = 8, spread = 3, type = COMBAT_LIFEDRAIN},
	{name = "speed", interval = 2000, chance = 5, radius = 3, effect = CONST_ME_POFF, target = false, speed = -700, duration = 12000},
	{name = "speed", interval = 2000, chance = 15, effect = CONST_ME_ICEATTACK, target = false, length = 7, spread = 3, speed = -850, duration = 18000},
	{name = "combat", interval = 2000, chance = 5, minDamage = -60, maxDamage = -120, radius = 3, effect = CONST_ME_ICETORNADO, target = false, type = COMBAT_ICEDAMAGE},
	{name = "combat", interval = 2000, chance = 10, minDamage = -100, maxDamage = -240, radius = 4, effect = CONST_ME_ICEAREA, target = true, type = COMBAT_ICEDAMAGE},
	{name = "combat", interval = 2000, chance = 5, minDamage = 0, maxDamage = -220, effect = CONST_ME_POFF, target = false, length = 1, spread = 0, type = COMBAT_PHYSICALDAMAGE},
	{name = "speed", interval = 2000, chance = 20, radius = 4, effect = CONST_ME_ICEAREA, target = true, speed = -600, duration = 12000},
}

monster.defenses = {
	defense = 45,
	armor = 38,
	{name = "combat", interval = 2000, chance = 10, minDamage = 150, maxDamage = 200, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
	{name = "speed", interval = 2000, chance = 15, effect = CONST_ME_REDSHIMMER, speed = 290, duration = 5000},
}

monster.elements = {
	{type = COMBAT_PHYSICALDAMAGE, percent = 10},
	{type = COMBAT_DEATHDAMAGE, percent = 10},
}

monster.immunities = {
	{type = "paralyze", combat = false, condition = true},
	{type = "invisible", combat = false, condition = true},
	{type = "fire", combat = true, condition = true},
	{type = "ice", combat = true, condition = true},
	{type = "earth", combat = true, condition = true},
}


mType:register(monster)