local mType = Game.createMonsterType("Colerian The Barbarian")
local monster = {}

monster.description = "Colerian the Barbarian"
monster.experience = 90
monster.outfit = {
	lookType = 253,
	lookHead = 76,
	lookBody = 115,
	lookLegs = 115,
	lookFeet = 43,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 7349
monster.health = 265
monster.maxHealth = 265
monster.race = "blood"
monster.speed = 190
monster.manaCost = 0
monster.maxSummons = 0

monster.flags = {
	summonable = false,
	attackable = true,
	hostile = true,
	illusionable = false,
	convinceable = false,
	pushable = false,
	canPushItems = true,
	targetDistance = 1,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = false,
}

monster.voices = {
	interval = 2000,
	chance = 5,
	{text = "Flee, coward!", yell = false},
	{text = "You will lose!", yell = false},
	{text = "Yeehaawh", yell = false},
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -60, target = false},
	{name = "combat", interval = 1000, chance = 80, minDamage = 0, maxDamage = -40, range = 5, radius = 1, shootEffect = CONST_ANI_PIERCINGBOLT, target = true, type = COMBAT_PHYSICALDAMAGE},
}

monster.defenses = {
	defense = 0,
	armor = 8,
}

monster.elements = {
	{type = COMBAT_ENERGYDAMAGE, percent = 20},
	{type = COMBAT_EARTHDAMAGE, percent = -20},
}

monster.immunities = {
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)