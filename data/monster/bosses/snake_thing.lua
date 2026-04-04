local mType = Game.createMonsterType("Snake Thing")
local monster = {}

monster.description = "Snake Thing"
monster.experience = 8400
monster.outfit = {
	lookType = 220,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 0
monster.health = 70000
monster.maxHealth = 70000
monster.race = "venom"
monster.speed = 240
monster.manaCost = 0
monster.maxSummons = 0

monster.changeTarget = {
	interval = 2000,
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
	staticAttackChance = 90,
	runHealth = 0,
	canWalkOnEnergy = true,
	canWalkOnFire = true,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 2000,
	chance = 10,
	{text = "POWER! I SEED MORE POWER!", yell = true},
}

monster.attacks = {
	{name = "melee", minDamage = 0, maxDamage = -400, interval = 2000, target = false},
	{name = "combat", type = COMBAT_EARTHDAMAGE, minDamage = -500, maxDamage = -500, interval = 2000, chance = 35, length = 8, spread = 3, target = false, effect = CONST_ME_POISON},
	{name = "combat", type = COMBAT_MANADRAIN, minDamage = -2398, maxDamage = -2398, interval = 2000, chance = 20, length = 8, spread = 0, target = false, effect = CONST_ME_REDNOTE},
	{name = "condition", type = CONDITION_POISON, interval = 2000, chance = 30, tick = 4000, minDamage = -30, maxDamage = -60, radius = 6, effect = CONST_ME_POISON, target = false},
}

monster.defenses = {
	defense = 30,
	armor = 45,
	{name = "combat", interval = 2000, chance = 25, minDamage = 150, maxDamage = 450, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
}

monster.elements = {
	{type = COMBAT_ICEDAMAGE, percent = 20},
	{type = COMBAT_FIREDAMAGE, percent = -10},
	{type = COMBAT_ENERGYDAMAGE, percent = -10},
}

monster.immunities = {
	{type = "paralyze", combat = false, condition = true},
	{type = "invisible", combat = false, condition = true},
	{type = "earth", combat = true, condition = true},
}


mType:register(monster)