local mType = Game.createMonsterType("Diseased Dan")
local monster = {}

monster.description = "a diseased Dan"
monster.experience = 300
monster.outfit = {
	lookType = 299,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 8951
monster.health = 800
monster.maxHealth = 800
monster.race = "venom"
monster.speed = 300
monster.manaCost = 0
monster.maxSummons = 0

monster.changeTarget = {
	interval = 60000,
	chance = 0
}

monster.flags = {
	summonable = false,
	attackable = true,
	hostile = true,
	illusionable = false,
	convinceable = false,
	pushable = false,
	canPushItems = true,
	staticAttackChance = 95,
	targetDistance = 1,
	runHealth = 1,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = false,
}

monster.voices = {
	interval = 2000,
	chance = 5,
	{text = "Where... Where am I?", yell = false},
	{text = "Is that you, Tom?", yell = false},
	{text = "Phew, what an awful smell ... oh, that's me.", yell = false},
}

monster.loot = {
	{id = 2148, chance = 28000, maxCount = 65}, -- gold coin
	{id = 2148, chance = 28000, maxCount = 64}, -- gold coin
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -207, target = false, condition = {type = CONDITION_POISON, startDamage = 4, interval = 2000}},
	{name = "combat", interval = 2000, chance = 15, minDamage = -90, maxDamage = -140, range = 7, effect = CONST_ME_REDSHIMMER, target = true, type = COMBAT_LIFEDRAIN},
	{name = "combat", interval = 1000, chance = 40, minDamage = -100, maxDamage = -175, radius = 2, shootEffect = CONST_ANI_SMALLEARTH, target = true, type = COMBAT_PHYSICALDAMAGE},
	{name = "speed", interval = 3000, chance = 40, range = 7, effect = CONST_ME_REDSHIMMER, target = true, speed = -900, duration = 20000},
}

monster.defenses = {
	defense = 15,
	armor = 10,
	{name = "speed", interval = 10000, chance = 40, effect = CONST_ME_GREENSHIMMER, speed = 310, duration = 20000},
	{name = "combat", interval = 5000, chance = 60, minDamage = 50, maxDamage = 80, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
}

monster.elements = {
	{type = COMBAT_PHYSICALDAMAGE, percent = -20},
	{type = COMBAT_ENERGYDAMAGE, percent = -10},
	{type = COMBAT_EARTHDAMAGE, percent = 30},
	{type = COMBAT_ICEDAMAGE, percent = -5},
	{type = COMBAT_HOLYDAMAGE, percent = -5},
	{type = COMBAT_DEATHDAMAGE, percent = 5},
	{type = COMBAT_FIREDAMAGE, percent = 85},
}

monster.immunities = {
	{type = "paralyze", combat = false, condition = true},
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)