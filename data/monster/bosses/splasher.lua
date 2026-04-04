local mType = Game.createMonsterType("Splasher")
local monster = {}

monster.description = "Splasher"
monster.experience = 500
monster.outfit = {
	lookType = 72,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6064
monster.health = 1700
monster.maxHealth = 1700
monster.race = "blood"
monster.speed = 520
monster.manaCost = 0
monster.maxSummons = 0

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
	staticAttackChance = 90,
	targetDistance = 1,
	runHealth = 0,
	canWalkOnEnergy = true,
	canWalkOnFire = true,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 2000,
	chance = 7,
	{text = "Qua hah tsh!", yell = false},
	{text = "Teech tsha tshul!", yell = false},
	{text = "Quara tsha Fach!", yell = false},
	{text = "Tssssha Quara!", yell = false},
	{text = "Blubber.", yell = false},
	{text = "Blup.", yell = false},
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -109, target = false, condition = {type = CONDITION_POISON, startDamage = 5, interval = 2000}},
	{name = "combat", interval = 2000, chance = 8, minDamage = -106, maxDamage = -169, range = 7, effect = CONST_ME_REDSHIMMER, target = true, type = COMBAT_LIFEDRAIN},
	{name = "combat", interval = 2000, chance = 7, minDamage = -162, maxDamage = -228, effect = CONST_ME_GREENSPARK, target = false, length = 8, spread = 3, type = COMBAT_LIFEDRAIN},
	{name = "combat", interval = 2000, chance = 9, minDamage = -134, maxDamage = -148, effect = CONST_ME_BUBBLES, target = false, length = 8, spread = 0, type = COMBAT_ICEDAMAGE},
	{name = "combat", interval = 2000, chance = 12, minDamage = -101, maxDamage = -149, radius = 3, effect = CONST_ME_BUBBLES, target = false, type = COMBAT_ICEDAMAGE},
	{name = "speed", interval = 2000, chance = 20, range = 1, effect = CONST_ME_REDSHIMMER, target = false, speed = -300, duration = 3000},
}

monster.defenses = {
	defense = 15,
	armor = 15,
	{name = "combat", interval = 2000, chance = 10, minDamage = 100, maxDamage = 120, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
}

monster.elements = {
	{type = COMBAT_ENERGYDAMAGE, percent = -25},
	{type = COMBAT_EARTHDAMAGE, percent = -15},
}

monster.immunities = {
	{type = "ice", combat = true, condition = true},
	{type = "fire", combat = true, condition = true},
	{type = "drown", combat = true, condition = true},
	{type = "paralyze", combat = false, condition = true},
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)