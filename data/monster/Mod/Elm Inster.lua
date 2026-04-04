local mType = Game.createMonsterType("Elm Inster")
local monster = {}

monster.description = "a Elm Inster"
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
monster.health = 2000
monster.maxHealth = 2000
monster.race = "undead"
monster.speed = 300
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
	canPushItems = true,
	canPushCreatures = true,
	staticAttackChance = 98,
	targetDistance = 1,
	runHealth = 100,
	canWalkOnEnergy = true,
	canWalkOnFire = true,
	canWalkOnPoison = true
}

monster.attacks = {
	{name = "melee", interval = 200, chance = 45, minDamage = -30, maxDamage = -300, skill = 153, target = false},
	{name = "combat", interval = 2000, chance = 15, minDamage = -30, maxDamage = -190, skill = 153, target = false, type = COMBAT_EARTHDAMAGE},
	{name = "combat", interval = 2000, chance = 15, minDamage = -30, maxDamage = -200, radius = 3, effect = CONST_ME_ICEAREA, target = false, type = COMBAT_ICEDAMAGE},
	{name = "combat", interval = 1000, chance = 30, minDamage = -50, maxDamage = -150, range = 7, shootEffect = CONST_ANI_ICE, target = true, type = COMBAT_ICEDAMAGE},
}

monster.defenses = {
	defense = 95,
	armor = 78,
	{name = "combat", interval = 3000, chance = 16, minDamage = 250, maxDamage = 600, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
	{name = "combat", interval = 2000, chance = 15, effect = CONST_ME_EXETA_RES, type = COMBAT_NONE},
}

monster.elements = {
	{type = COMBAT_PHYSICALDAMAGE, percent = 30},
	{type = COMBAT_DEATHDAMAGE, percent = 10},
	{type = COMBAT_ENERGYDAMAGE, percent = 10},
	{type = COMBAT_EARTHDAMAGE, percent = 10},
	{type = COMBAT_FIREDAMAGE, percent = 10},
	{type = COMBAT_ICEDAMAGE, percent = 10},
	{type = COMBAT_HOLYDAMAGE, percent = 10},
}

monster.immunities = {
	{type = "paralyze", combat = false, condition = true},
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)