local mType = Game.createMonsterType("Thornback Tortoise")
local monster = {}

monster.description = "a thornback tortoise"
monster.experience = 150
monster.outfit = {
	lookType = 198,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6073
monster.health = 300
monster.maxHealth = 300
monster.race = "blood"
monster.speed = 200
monster.manaCost = 490
monster.maxSummons = 0

monster.changeTarget = {
	interval = 5000,
	chance = 0
}

monster.flags = {
	summonable = false,
	attackable = true,
	hostile = true,
	illusionable = false,
	convinceable = true,
	pushable = false,
	canPushItems = true,
	canPushCreatures = true,
	staticAttackChance = 90,
	targetDistance = 1,
	runHealth = 0,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = false,
}

monster.loot = {
	{id = 2143, chance = 1600}, -- white pearl
	{id = 2144, chance = 800}, -- black pearl
	{id = 2148, chance = 89500, maxCount = 48}, -- gold coin
	{id = 2391, chance = 260}, -- war hammer
	{id = 2667, chance = 10800, maxCount = 2}, -- fish
	{id = 2787, chance = 1200}, -- white mushroom
	{id = 2789, chance = 700}, -- brown mushroom
	{id = 5678, chance = 790, maxCount = 3}, -- tortoise egg
	{id = 5899, chance = 800}, -- turtle shell
	{id = 7618, chance = 1600}, -- health potion
	{id = 10560, chance = 15980}, -- thorn
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -110, target = false, condition = {type = CONDITION_POISON, startDamage = 40, interval = 2000}},
}

monster.defenses = {
	defense = 40,
	armor = 24,
}

monster.elements = {
	{type = COMBAT_PHYSICALDAMAGE, percent = 45},
	{type = COMBAT_EARTHDAMAGE, percent = 20},
	{type = COMBAT_ICEDAMAGE, percent = 20},
	{type = COMBAT_FIREDAMAGE, percent = -10},
}


mType:register(monster)