local mType = Game.createMonsterType("Snake")
local monster = {}

monster.description = "a snake"
monster.experience = 10
monster.outfit = {
	lookType = 28,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 2817
monster.health = 15
monster.maxHealth = 15
monster.race = "blood"
monster.speed = 120
monster.manaCost = 205
monster.maxSummons = 0

monster.changeTarget = {
	interval = 4000,
	chance = 0
}

monster.flags = {
	summonable = true,
	attackable = true,
	hostile = true,
	illusionable = true,
	convinceable = false,
	pushable = true,
	canPushItems = false,
	canPushCreatures = false,
	targetDistance = 1,
	staticAttackChance = 90,
	runHealth = 0,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Zzzzzzt", yell = false},
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -8, target = false, condition = {type = CONDITION_POISON, startDamage = 15, interval = 2000}},
}

monster.defenses = {
	defense = 5,
	armor = 0,
}

monster.elements = {
	{type = COMBAT_EARTHDAMAGE, percent = 5},
	{type = COMBAT_ENERGYDAMAGE, percent = 20},
	{type = COMBAT_FIREDAMAGE, percent = -10},
	{type = COMBAT_ICEDAMAGE, percent = -10},
}


mType:register(monster)