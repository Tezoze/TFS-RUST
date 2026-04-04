local mType = Game.createMonsterType("Man In The Cave")
local monster = {}

monster.description = "man in the cave"
monster.experience = 777
monster.outfit = {
	lookType = 128,
	lookHead = 77,
	lookBody = 59,
	lookLegs = 20,
	lookFeet = 116,
	lookAddons = 1,
	lookMount = 0
}

monster.corpse = 6080
monster.health = 485
monster.maxHealth = 485
monster.race = "blood"
monster.speed = 210
monster.manaCost = 0
monster.maxSummons = 2

monster.changeTarget = {
	interval = 5000,
	chance = 8
}

monster.flags = {
	summonable = false,
	attackable = true,
	hostile = true,
	illusionable = false,
	convinceable = false,
	pushable = false,
	canPushItems = true,
	canPushCreatures = false,
	targetDistance = 1,
	staticAttackChance = 90,
	runHealth = 50,
	canWalkOnEnergy = true,
	canWalkOnFire = true,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 2000,
	chance = 5,
	{text = "THE MONKS ARE MINE!", yell = true},
	{text = "I will rope you up! All of you!", yell = false},
	{text = "You have been roped up!", yell = false},
	{text = "A MIC to rule them all!", yell = false},
}

monster.loot = {
	{id = 2120, chance = 100000, maxCount = 3}, -- rope
	{id = 7386, chance = 38000}, -- mercenary sword
	{id = 5913, chance = 30000}, -- brown piece of cloth
	{id = 2148, chance = 30000, maxCount = 39}, -- gold coin
	{id = 7458, chance = 15000}, -- fur cap
	{id = 7290, chance = 8000}, -- shard
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -62, target = false},
	{name = "combat", interval = 2000, chance = 20, minDamage = 0, maxDamage = -95, range = 7, shootEffect = CONST_ANI_SMALLSTONE, target = true, type = COMBAT_PHYSICALDAMAGE},
}

monster.defenses = {
	defense = 15,
	armor = 15,
	{name = "speed", interval = 2000, chance = 12, effect = CONST_ME_REDSHIMMER, speed = 250, duration = 4000},
	{name = "combat", interval = 2000, chance = 25, minDamage = 10, maxDamage = 50, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
}

monster.immunities = {
	{type = "invisible", combat = false, condition = true},
}

monster.summons = {
	{name = "Monk", chance = 20, interval = 2000, max = 2},
}

mType:register(monster)