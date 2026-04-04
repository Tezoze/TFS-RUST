local mType = Game.createMonsterType("Sr. Archimonde")
local monster = {}

monster.description = "a Sr. Archimonde"
monster.experience = 0
monster.outfit = {
	lookType = 75,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.health = 20000
monster.maxHealth = 20000
monster.race = "undead"
monster.speed = 450
monster.manaCost = 2000
monster.maxSummons = 0

monster.changeTarget = {
	interval = 4000,
	chance = 20
}

monster.flags = {
	summonable = false,
	attackable = true,
	hostile = true,
	illusionable = true,
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
	chance = 5,
	{text = "Everything is lost little bug!!", yell = true},
}

monster.attacks = {
	{name = "melee", interval = 2000, skill = 160, attack = 300, target = false},
	{name = "combat", interval = 2000, chance = 12, minDamage = 0, maxDamage = -1100, range = 7, shootEffect = CONST_ANI_LARGEROCK, effect = CONST_ME_EXPLOSIONAREA, target = true, type = COMBAT_PHYSICALDAMAGE},
	{name = "combat", interval = 2000, chance = 15, minDamage = -30, maxDamage = -190, skill = 153, target = false, type = COMBAT_HOLYDAMAGE},
	{name = "combat", interval = 2000, chance = 15, minDamage = -30, maxDamage = -200, radius = 3, effect = CONST_ME_EXEVO_MAS_SAN, target = false, type = COMBAT_PHYSICALDAMAGE},
}

monster.defenses = {
	defense = 95,
	armor = 55,
	{name = "combat", interval = 3000, chance = 35, minDamage = 550, maxDamage = 1000, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
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