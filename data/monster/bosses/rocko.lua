local mType = Game.createMonsterType("Rocko")
local monster = {}

monster.description = "Rocko"
monster.experience = 3400
monster.outfit = {
	lookType = 67,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6005
monster.health = 10000
monster.maxHealth = 10000
monster.race = "blood"
monster.speed = 180
monster.manaCost = 0
monster.maxSummons = 0

monster.changeTarget = {
	interval = 2000,
	chance = 9
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

monster.attacks = {
	{name = "melee", interval = 2000, skill = 28, attack = 100, target = false},
	{name = "combat", interval = 2000, chance = 20, minDamage = 0, maxDamage = -125, effect = CONST_ME_POISON, target = false, length = 8, spread = 0, type = COMBAT_EARTHDAMAGE},
	{name = "combat", interval = 2000, chance = 20, minDamage = 0, maxDamage = -411, effect = CONST_ME_GROUNDSHAKER, target = false, length = 8, spread = 0, type = COMBAT_PHYSICALDAMAGE},
}

monster.defenses = {
	defense = 18,
	armor = 10,
}

monster.elements = {
	{type = COMBAT_HOLYDAMAGE, percent = -10},
	{type = COMBAT_PHYSICALDAMAGE, percent = 15},
	{type = COMBAT_FIREDAMAGE, percent = 30},
	{type = COMBAT_ENERGYDAMAGE, percent = 25},
	{type = COMBAT_ICEDAMAGE, percent = 20},
}

monster.immunities = {
	{type = "paralyze", combat = false, condition = true},
	{type = "earth", combat = true, condition = true},
	{type = "death", combat = true, condition = true},
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)