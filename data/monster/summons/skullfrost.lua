local mType = Game.createMonsterType("Skullfrost")
local monster = {}

monster.description = "a Skullfrost"
monster.experience = 0
monster.outfit = {
	lookType = 300,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 8955
monster.health = 10000
monster.maxHealth = 10000
monster.race = "undead"
monster.speed = 500
monster.manaCost = 1000
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
	targetDistance = 3,
	runHealth = 9000,
	canWalkOnEnergy = true,
	canWalkOnFire = true,
	canWalkOnPoison = true
}

monster.attacks = {
	{name = "combat", interval = 200, chance = 25, minDamage = -100, maxDamage = -280, target = false, type = COMBAT_DEATHDAMAGE},
	{name = "combat", interval = 500, chance = 10, minDamage = -200, maxDamage = -500, effect = CONST_ME_MORTAREA, target = true, length = 6, spread = 0, type = COMBAT_DEATHDAMAGE},
	{name = "combat", interval = 500, chance = 10, minDamage = -50, maxDamage = -500, radius = 7, effect = CONST_ME_MORTAREA, target = false, type = COMBAT_DEATHDAMAGE},
	{name = "combat", interval = 2000, chance = 15, minDamage = -100, maxDamage = -240, radius = 7, effect = CONST_ME_ICEAREA, target = true, type = COMBAT_ICEDAMAGE},
	{name = "combat", interval = 2000, chance = 15, minDamage = -60, maxDamage = -400, radius = 7, effect = CONST_ME_ICETORNADO, target = false, type = COMBAT_ICEDAMAGE},
}

monster.defenses = {
	defense = 95,
	armor = 55,
	{name = "combat", interval = 3000, chance = 25, minDamage = 80, maxDamage = 400, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
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