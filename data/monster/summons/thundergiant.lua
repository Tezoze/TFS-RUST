local mType = Game.createMonsterType("Thundergiant")
local monster = {}

monster.description = "a Thundergiant"
monster.experience = 0
monster.outfit = {
	lookType = 149,
	lookHead = 94,
	lookBody = 77,
	lookLegs = 96,
	lookFeet = 0,
	lookAddons = 3,
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
	convinceable = true,
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
	{name = "melee", interval = 200, chance = 25, minDamage = -100, maxDamage = -310, target = false},
	{name = "combat", interval = 500, chance = 23, minDamage = -100, maxDamage = -500, effect = CONST_ME_LIFEDRAIN, target = false, length = 3, spread = 3, type = COMBAT_PHYSICALDAMAGE},
	{name = "combat", interval = 2000, chance = 15, minDamage = -75, maxDamage = -705, range = 7, shootEffect = CONST_ANI_ENERGYBALL, effect = CONST_ME_ENERGY, target = true, length = 2, type = COMBAT_ENERGYDAMAGE},
}

monster.defenses = {
	defense = 95,
	armor = 55,
	{name = "combat", interval = 3000, chance = 25, minDamage = 80, maxDamage = 600, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
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