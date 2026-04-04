local mType = Game.createMonsterType("Tremorak")
local monster = {}

monster.description = "Tremorak"
monster.experience = 1300
monster.outfit = {
	lookType = 285,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 8933
monster.health = 10000
monster.maxHealth = 10000
monster.race = "undead"
monster.speed = 290
monster.manaCost = 0
monster.maxSummons = 0

monster.changeTarget = {
	interval = 2000,
	chance = 5
}

monster.flags = {
	summonable = false,
	attackable = true,
	hostile = true,
	illusionable = false,
	convinceable = false,
	pushable = false,
	canPushItems = true,
	staticAttackChance = 80,
	targetDistance = 1,
	runHealth = 1,
	canWalkOnEnergy = true,
	canWalkOnFire = true,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 2000,
	chance = 9,
	{text = "*STOMP STOMP*", yell = true},
}

monster.attacks = {
	{name = "melee", interval = 2000, target = false},
	{name = "combat", type = COMBAT_EARTHDAMAGE, minDamage = 0, maxDamage = -255, interval = 2000, chance = 16, radius = 7, target = false, effect = CONST_ME_GROUNDSHAKER},
	{name = "combat", type = COMBAT_EARTHDAMAGE, minDamage = 0, maxDamage = -405, interval = 2000, chance = 16, length = 8, spread = 0, target = false, effect = CONST_ME_GROUNDSHAKER},
	{name = "condition", type = CONDITION_POISON, interval = 2000, chance = 16, tick = 4000, minDamage = -20, maxDamage = -20, duration = 16000, range = 7, shootEffect = CONST_ANI_POISON, target = true},
}

monster.defenses = {
	defense = 30,
	armor = 30,
	{name = "combat", interval = 2000, chance = 16, minDamage = 75, maxDamage = 200, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
}

monster.elements = {
	{type = COMBAT_DEATHDAMAGE, percent = 45},
	{type = COMBAT_ICEDAMAGE, percent = 20},
	{type = COMBAT_PHYSICALDAMAGE, percent = 50},
	{type = COMBAT_HOLYDAMAGE, percent = 50},
	{type = COMBAT_FIREDAMAGE, percent = -15},
	{type = COMBAT_ENERGYDAMAGE, percent = 85},
}

monster.immunities = {
	{type = "earth", combat = true, condition = true},
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)