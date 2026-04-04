local mType = Game.createMonsterType("Son of Verminor")
local monster = {}

monster.description = "a son of verminor"
monster.experience = 5900
monster.outfit = {
	lookType = 19,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 1490
monster.health = 8500
monster.maxHealth = 8500
monster.race = "venom"
monster.speed = 240
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
	targetDistance = 1,
	staticAttackChance = 70,
	runHealth = 0,
	canWalkOnEnergy = true,
	canWalkOnFire = false,
	canWalkOnPoison = true,
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Blubb.", yell = false},
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -473, target = false, condition = {type = CONDITION_POISON, startDamage = 450, interval = 2000}},
	{name = "combat", interval = 2000, chance = 20, minDamage = -150, maxDamage = -200, range = 7, shootEffect = CONST_ANI_POISON, target = true, type = COMBAT_EARTHDAMAGE},
	{name = "combat", interval = 2000, chance = 15, minDamage = -350, maxDamage = -390, radius = 3, effect = CONST_ME_POISON, target = false, type = COMBAT_EARTHDAMAGE},
	{name = "combat", interval = 2000, chance = 15, minDamage = -220, maxDamage = -270, radius = 3, effect = CONST_ME_SMALLCLOUDS, target = false, type = COMBAT_DEATHDAMAGE},
}

monster.defenses = {
	defense = 0,
	armor = 48,
	{name = "combat", interval = 2000, chance = 20, minDamage = 250, maxDamage = 350, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
	{name = "outfit", interval = 5000, chance = 10, effect = CONST_ME_BLUESHIMMER, monster = "rat", duration = 6000},
	{name = "outfit", interval = 5000, chance = 10, effect = CONST_ME_BLUESHIMMER, monster = "larva", duration = 6000},
	{name = "outfit", interval = 5000, chance = 10, effect = CONST_ME_BLUESHIMMER, monster = "scorpion", duration = 6000},
	{name = "outfit", interval = 5000, chance = 10, effect = CONST_ME_BLUESHIMMER, monster = "slime", duration = 6000},
}

monster.elements = {
	{type = COMBAT_FIREDAMAGE, percent = 10},
	{type = COMBAT_ENERGYDAMAGE, percent = 20},
}

monster.immunities = {
	{type = "earth", combat = true, condition = true},
	{type = "paralyze", combat = false, condition = true},
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)