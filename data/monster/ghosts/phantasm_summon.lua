local mType = Game.createMonsterType("Phantasm")
local monster = {}

monster.description = "a phantasm"
monster.experience = 4400
monster.outfit = {
	lookType = 241,
	lookHead = 20,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6344
monster.health = 3950
monster.maxHealth = 3950
monster.race = "undead"
monster.speed = 340
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
	illusionable = true,
	convinceable = false,
	pushable = false,
	canPushItems = true,
	canPushCreatures = true,
	targetDistance = 1,
	staticAttackChance = 90,
	runHealth = 0,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = false,
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Oh my, you forgot to put your pants on!", yell = false},
	{text = "Weeheeheeheehee!", yell = false},
	{text = "Its nothing but a dream.", yell = false},
	{text = "Dream a little dream with me!", yell = false},
	{text = "Give in.", yell = false},
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -50, target = false},
	{name = "combat", interval = 2000, chance = 15, minDamage = -50, maxDamage = -80, range = 7, shootEffect = CONST_ANI_SUDDENDEATH, effect = CONST_ME_SMALLCLOUDS, target = true, type = COMBAT_DEATHDAMAGE},
	{name = "combat", interval = 2000, chance = 10, minDamage = -5, maxDamage = -80, radius = 3, effect = CONST_ME_YELLOWBUBBLE, target = false, type = COMBAT_MANADRAIN},
	{name = "combat", interval = 2000, chance = 10, target = false, type = COMBAT_PHYSICALDAMAGE},
	{name = "combat", interval = 2000, chance = 5, radius = 5, effect = CONST_ME_REDSHIMMER, target = false, type = COMBAT_PHYSICALDAMAGE},
}

monster.defenses = {
	defense = 0,
	armor = 0,
	{name = "combat", interval = 2000, chance = 30, minDamage = 40, maxDamage = 65, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
	{name = "speed", interval = 2000, chance = 10, effect = CONST_ME_REDSHIMMER, speed = 500, duration = 6000},
	{name = "invisible", interval = 4000, chance = 15, effect = CONST_ME_BLUESHIMMER},
}

monster.elements = {
	{type = COMBAT_EARTHDAMAGE, percent = 20},
	{type = COMBAT_ICEDAMAGE, percent = 20},
	{type = COMBAT_HOLYDAMAGE, percent = -10},
	{type = COMBAT_FIREDAMAGE, percent = -10},
	{type = COMBAT_ENERGYDAMAGE, percent = -10},
}

monster.immunities = {
	{type = "paralyze", combat = false, condition = true},
	{type = "invisible", combat = false, condition = true},
	{type = "death", combat = true, condition = true},
	{type = "physical", combat = true, condition = true},
}


mType:register(monster)