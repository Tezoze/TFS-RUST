local mType = Game.createMonsterType("Azerus")
local monster = {}

monster.description = "Azerus"
monster.experience = 6000
monster.outfit = {
	lookType = 309,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 0
monster.health = 26000
monster.maxHealth = 26000
monster.race = "blood"
monster.speed = 286
monster.manaCost = 0
monster.maxSummons = 10

monster.changeTarget = {
	interval = 5000,
	chance = 8
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
	staticAttackChance = 85,
	runHealth = 0,
	canWalkOnEnergy = true,
	canWalkOnFire = true,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 5000,
	chance = 12,
	{text = "The ultimate will finally consume this unworthy existence!", yell = false},
	{text = "My masters and I will tear down barriers and join the ultimate in its realm!", yell = false},
	{text = "The power of the Yalahari will all be mine!", yell = false},
	{text = "He who has returned from beyond has taught me secrets you can't even grasp!", yell = false},
	{text = "You can't hope to penetrate my shields!", yell = false},
	{text = "Do you really think you could beat me?", yell = false},
	{text = "We will open the rift for a new time to come!", yell = false},
	{text = "The end of times has come!", yell = false},
	{text = "The great machinator will make his entrance soon!", yell = false},
	{text = "You might scratch my shields but they will never break!", yell = false},
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -900, target = false},
	{name = "combat", interval = 2000, chance = 15, minDamage = 0, maxDamage = -3800, range = 7, radius = 4, shootEffect = CONST_ANI_FIRE, effect = CONST_ME_BIGCLOUDS, target = true, type = COMBAT_MANADRAIN},
	{name = "combat", interval = 2000, chance = 20, minDamage = 0, maxDamage = -524, range = 7, shootEffect = CONST_ANI_SUDDENDEATH, effect = CONST_ME_MORTAREA, target = true, type = COMBAT_DEATHDAMAGE},
	{name = "combat", interval = 2000, chance = 10, minDamage = -300, maxDamage = -1050, effect = CONST_ME_FIREATTACK, target = false, length = 8, spread = 0, type = COMBAT_FIREDAMAGE},
}

monster.defenses = {
	defense = 65,
	armor = 40,
	{name = "combat", interval = 2000, chance = 11, minDamage = 401, maxDamage = 499, effect = CONST_ME_GREENSHIMMER, type = COMBAT_HEALING},
}

monster.elements = {
	{type = COMBAT_EARTHDAMAGE, percent = 15},
	{type = COMBAT_DEATHDAMAGE, percent = 15},
	{type = COMBAT_HOLYDAMAGE, percent = 15},
	{type = COMBAT_FIREDAMAGE, percent = 15},
	{type = COMBAT_ENERGYDAMAGE, percent = 15},
	{type = COMBAT_ICEDAMAGE, percent = 15},
}

monster.immunities = {
	{type = "paralyze", combat = false, condition = true},
	{type = "invisible", combat = false, condition = true},
}

monster.summons = {
	{name = "Rift Worm", chance = 10, interval = 8000, max = 8},
	{name = "Rift Brood", chance = 10, interval = 8000, max = 8},
	{name = "Rift Scythe", chance = 10, interval = 8000, max = 8},
	{name = "War Golem", chance = 10, interval = 8000, max = 5},
}

mType:register(monster)