local mType = Game.createMonsterType("Mutated Zalamon")
local monster = {}

monster.description = "Mutated Zalamon"
monster.experience = 10980
monster.outfit = {
	lookType = 356,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 12385
monster.health = 155000
monster.maxHealth = 155000
monster.race = "venom"
monster.speed = 238
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
	staticAttackChance = 90,
	targetDistance = 1,
	runHealth = 0,
	canWalkOnEnergy = true,
	canWalkOnFire = true,
	canWalkOnPoison = true
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -400, target = false},
	{name = "combat", interval = 2000, chance = 15, minDamage = 0, maxDamage = -815, range = 7, shootEffect = CONST_ANI_POISON, effect = CONST_ME_POISON, target = true, type = COMBAT_EARTHDAMAGE},
	{name = "combat", interval = 2000, chance = 10, minDamage = -100, maxDamage = -300, radius = 4, shootEffect = CONST_ANI_POISON, effect = CONST_ME_POISON, target = true, type = COMBAT_EARTHDAMAGE},
	{name = "speed", interval = 4000, chance = 20, range = 7, shootEffect = CONST_ANI_POISON, target = true, speed = -350, duration = 12000},
}

monster.defenses = {
	defense = 65,
	armor = 70,
	{name = "combat", interval = 2000, chance = 9, minDamage = 20, maxDamage = 560, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
	{name = "outfit", interval = 2000, chance = 10, effect = CONST_ME_ENERGY, monster = "Lizard Snakecharmer", duration = 10000},
	{name = "outfit", interval = 2000, chance = 10, effect = CONST_ME_ENERGY, monster = "Lizard Abomination", duration = 10000},
	{name = "outfit", interval = 2000, chance = 10, effect = CONST_ME_ENERGY, monster = "Serpent Spawn", duration = 10000},
	{name = "outfit", interval = 2000, chance = 10, effect = CONST_ME_ENERGY, monster = "Draken Abomination", duration = 10000},
	{name = "outfit", interval = 2000, chance = 10, effect = CONST_ME_ENERGY, monster = "Mutated Zalamon", duration = 10000},
}

monster.elements = {
	{type = COMBAT_FIREDAMAGE, percent = -10},
	{type = COMBAT_ENERGYDAMAGE, percent = -10},
	{type = COMBAT_PHYSICALDAMAGE, percent = 5},
	{type = COMBAT_ICEDAMAGE, percent = 10},
}

monster.immunities = {
	{type = "earth", combat = true, condition = true},
	{type = "invisible", combat = false, condition = true},
	{type = "paralyze", combat = false, condition = true},
}


mType:register(monster)