local mType = Game.createMonsterType("Grovebeast")
local monster = {}

monster.description = "Grovebeast"
monster.experience = 0
monster.outfit = {
	lookType = 149,
	lookHead = 0,
	lookBody = 47,
	lookLegs = 105,
	lookFeet = 105,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6081
monster.health = 20000
monster.maxHealth = 20000
monster.race = "undead"
monster.speed = 500
monster.manaCost = 3000
monster.maxSummons = 0

monster.changeTarget = {
	interval = 4000,
	chance = 20
}

monster.flags = {
	summonable = false,
	attackable = true,
	hostile = false,
	illusionable = false,
	convinceable = false,
	pushable = true,
	canPushItems = false,
	canPushCreatures = true,
	staticAttackChance = 98,
	targetDistance = 1,
	runHealth = 9000,
	canWalkOnEnergy = true,
	canWalkOnFire = true,
	canWalkOnPoison = true
}

monster.attacks = {
	{name = "melee", interval = 200, chance = 45, minDamage = -100, maxDamage = -300, target = false},
	{name = "combat", interval = 1000, chance = 10, minDamage = 0, maxDamage = -800, effect = CONST_ME_STONES, target = true, spread = 0, type = COMBAT_EARTHDAMAGE},
	{name = "combat", interval = 2000, chance = 20, minDamage = -50, maxDamage = -200, range = 1, effect = CONST_ME_REDSHIMMER, target = true, type = COMBAT_LIFEDRAIN},
	{name = "combat", interval = 2000, chance = 15, minDamage = -250, maxDamage = -500, effect = CONST_ME_CARNIPHILA, target = false, length = 2, spread = 3, type = COMBAT_EARTHDAMAGE},
	{name = "combat", interval = 500, chance = 3, radius = 8, effect = CONST_ME_EXETA_RES, target = false, type = COMBAT_PHYSICALDAMAGE},
}

monster.defenses = {
	defense = 95,
	armor = 78,
	{name = "combat", interval = 3000, chance = 16, minDamage = 80, maxDamage = 600, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
	{name = "combat", interval = 100, chance = 15, effect = CONST_ME_EXETA_RES, type = COMBAT_NONE},
}

monster.elements = {
	{type = COMBAT_FIREDAMAGE, percent = 0},
	{type = COMBAT_EARTHDAMAGE, percent = 0},
	{type = COMBAT_ENERGYDAMAGE, percent = 0},
	{type = COMBAT_DEATHDAMAGE, percent = 0},
	{type = COMBAT_PHYSICALDAMAGE, percent = 0},
	{type = COMBAT_HOLYDAMAGE, percent = 0},
	{type = COMBAT_ICEDAMAGE, percent = 0},
}

monster.immunities = {
	{type = "paralyze", combat = false, condition = true},
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)