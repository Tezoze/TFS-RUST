local mType = Game.createMonsterType("Emberwing")
local monster = {}

monster.description = "a Emberwing"
monster.experience = 0
monster.outfit = {
	lookType = 73,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6080
monster.health = 15000
monster.maxHealth = 15000
monster.race = "undead"
monster.speed = 500
monster.manaCost = 2000
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
	targetDistance = 2,
	runHealth = 9000,
	canWalkOnEnergy = true,
	canWalkOnFire = true,
	canWalkOnPoison = true
}

monster.attacks = {
	{name = "combat", interval = 200, chance = 25, minDamage = -100, maxDamage = -190, target = false, type = COMBAT_HOLYDAMAGE},
	{name = "combat", interval = 500, chance = 15, minDamage = -200, maxDamage = -500, effect = CONST_ME_EXEVO_MAS_SAN, target = true, length = 10, spread = 10, type = COMBAT_PHYSICALDAMAGE},
	{name = "combat", interval = 500, chance = 16, minDamage = -50, maxDamage = -500, radius = 7, effect = CONST_ME_FIREAREA, target = false, type = COMBAT_PHYSICALDAMAGE},
	{name = "combat", interval = 2000, chance = 30, minDamage = -100, maxDamage = -240, radius = 4, effect = CONST_ME_EXEVO_MAS_SAN, target = true, type = COMBAT_PHYSICALDAMAGE},
}

monster.defenses = {
	defense = 95,
	armor = 55,
	{name = "combat", interval = 3000, chance = 35, minDamage = 80, maxDamage = 400, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
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