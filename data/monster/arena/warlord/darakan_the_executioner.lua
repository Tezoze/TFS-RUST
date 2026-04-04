local mType = Game.createMonsterType("Darakan the Executioner")
local monster = {}

monster.description = "Darakan the Executioner"
monster.experience = 1600
monster.outfit = {
	lookType = 255,
	lookHead = 78,
	lookBody = 114,
	lookLegs = 114,
	lookFeet = 114,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 7349
monster.health = 3480
monster.maxHealth = 3480
monster.race = "blood"
monster.speed = 205
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
	{text = "FIGHT LIKE A BARBARIAN!", yell = true},
	{text = "VICTORY IS MINE!", yell = true},
	{text = "I AM your father!", yell = false},
	{text = "To be the man you have to beat the man!", yell = false},
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -210, target = false},
	{name = "combat", interval = 1000, chance = 100, minDamage = -72, maxDamage = -130, range = 7, shootEffect = CONST_ANI_SPEAR, target = true, type = COMBAT_PHYSICALDAMAGE},
}

monster.defenses = {
	defense = 31,
	armor = 30,
	{name = "combat", interval = 6000, chance = 65, minDamage = 20, maxDamage = 50, effect = CONST_ME_BLUESHIMMER, type = COMBAT_HEALING},
}

monster.elements = {
	{type = COMBAT_ICEDAMAGE, percent = 1},
	{type = COMBAT_FIREDAMAGE, percent = -1},
}

monster.immunities = {
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)