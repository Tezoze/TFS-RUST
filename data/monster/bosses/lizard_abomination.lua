local mType = Game.createMonsterType("Lizard Abomination")
local monster = {}

monster.description = "a lizard abomination"
monster.experience = 9700
monster.outfit = {
	lookType = 364,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 0
monster.health = 95000
monster.maxHealth = 95000
monster.race = "blood"
monster.speed = 300
monster.manaCost = 0
monster.maxSummons = 0

monster.changeTarget = {
	interval = 2000,
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
	targetDistance = 1,
	staticAttackChance = 90,
	runHealth = 0,
	canWalkOnEnergy = true,
	canWalkOnFire = true,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 2000,
	chance = 10,
	{text = "NOOOO! NOW YOU HERETICS WILL FACE MY GODLY WRATH!", yell = true},
	{text = "RAAARRRR! I WILL DEVOL YOU!", yell = true},
	{text = "I WILL MAKE YOU ZHEE!", yell = true},
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -550, target = false},
	{name = "combat", interval = 2000, chance = 40, minDamage = 0, maxDamage = -980, radius = 3, effect = CONST_ME_GREENSPARK, target = false, type = COMBAT_EARTHDAMAGE},
	{name = "combat", interval = 2000, chance = 50, minDamage = -200, maxDamage = -300, effect = CONST_ME_REDSHIMMER, target = false, length = 8, spread = 0, type = COMBAT_LIFEDRAIN},
	{name = "speed", interval = 2000, chance = 20, radius = 3, effect = CONST_ME_POISON, target = false, speed = -400},
}

monster.defenses = {
	defense = 60,
	armor = 55,
	{name = "combat", interval = 2000, chance = 25, minDamage = 50, maxDamage = 350, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
}

monster.elements = {
	{type = COMBAT_PHYSICALDAMAGE, percent = 15},
	{type = COMBAT_FIREDAMAGE, percent = -10},
	{type = COMBAT_ENERGYDAMAGE, percent = -10},
	{type = COMBAT_ICEDAMAGE, percent = 20},
}

monster.immunities = {
	{type = "earth", combat = true, condition = true},
	{type = "invisibility", combat = false, condition = true},
}


mType:register(monster)