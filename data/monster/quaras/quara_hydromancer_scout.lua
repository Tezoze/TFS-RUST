local mType = Game.createMonsterType("Quara Hydromancer Scout")
local monster = {}

monster.description = "a quara hydromancer scout"
monster.experience = 800
monster.outfit = {
	lookType = 47,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6066
monster.health = 1100
monster.maxHealth = 1100
monster.race = "undead"
monster.speed = 190
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
	illusionable = false,
	convinceable = false,
	pushable = false,
	canPushItems = true,
	canPushCreatures = true,
	staticAttackChance = 90,
	targetDistance = 1,
	runHealth = 0,
	canWalkOnEnergy = false,
	canWalkOnPoison = false,
	canWalkOnFire = true,
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Qua hah tsh!", yell = false},
	{text = "Teech tsha tshul!", yell = false},
	{text = "Quara tsha Fach!", yell = false},
	{text = "Tssssha Quara!", yell = false},
	{text = "Blubber.", yell = false},
	{text = "Blup.", yell = false},
}

monster.loot = {
	{id = 2143, chance = 3850}, -- white pearl
	{id = 2144, chance = 2660}, -- black pearl
	{id = 2148, chance = 40000, maxCount = 90}, -- gold coin
	{id = 2148, chance = 49000, maxCount = 49}, -- gold coin
	{id = 2149, chance = 6000, maxCount = 2}, -- small emerald
	{id = 2189, chance = 890}, -- wand of cosmic energy
	{id = 2214, chance = 590}, -- ring of healing
	{id = 2425, chance = 1180}, -- obsidian lance
	{id = 2476, chance = 890}, -- knight armor
	{id = 2667, chance = 18930, maxCount = 2}, -- fish
	{id = 2670, chance = 8400, maxCount = 4}, -- shrimp
	{id = 5895, chance = 1630}, -- fish fin
	{id = 12444, chance = 12720}, -- quara eye
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -80, target = false, condition = {type = CONDITION_POISON, startDamage = 100, interval = 2000}},
	{name = "combat", interval = 2000, chance = 10, minDamage = -100, maxDamage = -180, effect = CONST_ME_GREENSPARK, target = false, length = 8, spread = 0, type = COMBAT_ICEDAMAGE},
	{name = "combat", interval = 2000, chance = 10, minDamage = -90, maxDamage = -150, radius = 3, effect = CONST_ME_BUBBLES, target = false, type = COMBAT_ICEDAMAGE},
	{name = "combat", interval = 2000, chance = 10, minDamage = -170, maxDamage = -240, effect = CONST_ME_BUBBLES, target = false, length = 8, spread = 0, type = COMBAT_LIFEDRAIN},
	{name = "combat", interval = 2000, chance = 15, minDamage = 0, maxDamage = -170, range = 7, effect = CONST_ME_REDSHIMMER, target = true, type = COMBAT_LIFEDRAIN},
	{name = "speed", interval = 2000, chance = 15, range = 7, effect = CONST_ME_REDSHIMMER, target = false, speed = -600, duration = 15000},
}

monster.defenses = {
	defense = 15,
	armor = 30,
	{name = "combat", interval = 2000, chance = 15, minDamage = 100, maxDamage = 120, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
}

monster.elements = {
	{type = COMBAT_ENERGYDAMAGE, percent = -10},
}

monster.immunities = {
	{type = "ice", combat = true, condition = true},
	{type = "fire", combat = true, condition = true},
	{type = "paralyze", combat = false, condition = true},
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)