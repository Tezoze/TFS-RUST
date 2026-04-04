local mType = Game.createMonsterType("Heoni")
local monster = {}

monster.description = "Heoni"
monster.experience = 515
monster.outfit = {
	lookType = 239,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 12546
monster.health = 900
monster.maxHealth = 900
monster.race = "blood"
monster.speed = 300
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
	runHealth = 300,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 2000,
	chance = 10,
	{text = "Shriiiek", yell = false},
}

monster.attacks = {
	{name = "melee", interval = 2000, target = false},
	{name = "condition", type = CONDITION_POISON, interval = 2000, chance = 15, tick = 4000, minDamage = -20, maxDamage = -240, length = 8, spread = 3, effect = CONST_ME_POISON, target = false},
	{name = "drunk", interval = 2000, chance = 13, length = 8, spread = 3, target = false, effect = CONST_ME_WHITENOTE},
}

monster.defenses = {
	defense = 18,
	armor = 25,
	{name = "combat", interval = 2000, chance = 11, minDamage = 76, maxDamage = 84, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
	{name = "speed", interval = 2000, chance = 10, effect = CONST_ME_REDSHIMMER, speed = 290, duration = 5000},
}

monster.elements = {
	{type = COMBAT_ENERGYDAMAGE, percent = 20},
	{type = COMBAT_ICEDAMAGE, percent = 10},
}

monster.immunities = {
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)