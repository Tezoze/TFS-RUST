local mType = Game.createMonsterType("The Keeper")
local monster = {}

monster.description = "The Keeper"
monster.experience = 3205
monster.outfit = {
	lookType = 220,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 12316
monster.health = 40000
monster.maxHealth = 40000
monster.race = "venom"
monster.speed = 290
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
	staticAttackChance = 80,
	runHealth = 275,
	canWalkOnEnergy = true,
	canWalkOnFire = true,
	canWalkOnPoison = true
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -250, target = false},
	{name = "speed", interval = 4000, chance = 20, range = 7, shootEffect = CONST_ANI_POISON, target = true, speed = -350, duration = 12000},
	{name = "combat", interval = 2000, chance = 20, minDamage = -80, maxDamage = -300, range = 7, shootEffect = CONST_ANI_POISON, target = true, type = COMBAT_EARTHDAMAGE},
	{name = "combat", interval = 2000, chance = 10, minDamage = -200, maxDamage = -500, effect = CONST_ME_REDNOTE, target = false, length = 8, spread = 0, type = COMBAT_LIFEDRAIN},
	{name = "outfit", interval = 2000, chance = 1, range = 7, effect = CONST_ME_BLUESHIMMER, target = false},
	{name = "combat", interval = 2000, chance = 10, minDamage = -200, maxDamage = -500, effect = CONST_ME_POISON, target = false, length = 8, spread = 3, type = COMBAT_EARTHDAMAGE},
}

monster.defenses = {
	defense = 35,
	armor = 35,
	{name = "combat", interval = 2000, chance = 15, minDamage = 250, maxDamage = 400, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
}

monster.elements = {
	{type = COMBAT_DEATHDAMAGE, percent = 50},
	{type = COMBAT_ICEDAMAGE, percent = 20},
	{type = COMBAT_FIREDAMAGE, percent = -10},
	{type = COMBAT_ENERGYDAMAGE, percent = -10},
}

monster.immunities = {
	{type = "earth", combat = true, condition = true},
	{type = "paralyze", combat = false, condition = true},
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)