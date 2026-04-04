local mType = Game.createMonsterType("Dreadbeast")
local monster = {}

monster.description = "a dreadbeast"
monster.experience = 250
monster.outfit = {
	lookType = 101,
	lookHead = 20,
	lookBody = 30,
	lookLegs = 40,
	lookFeet = 50,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 3031
monster.health = 800
monster.maxHealth = 800
monster.race = "undead"
monster.speed = 336
monster.manaCost = 800
monster.maxSummons = 0

monster.changeTarget = {
	interval = 60000,
	chance = 10
}

monster.flags = {
	summonable = false,
	attackable = true,
	hostile = true,
	illusionable = false,
	convinceable = true,
	pushable = false,
	canPushItems = true,
	staticAttackChance = 80,
	targetDistance = 1,
	runHealth = 0,
	canWalkOnEnergy = true,
	canWalkOnFire = true,
	canWalkOnPoison = true
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -100, target = false},
	{name = "combat", interval = 2000, chance = 20, minDamage = -150, maxDamage = -250, radius = 1, shootEffect = CONST_ANI_SUDDENDEATH, effect = CONST_ME_YELLOWENERGY, target = true, type = COMBAT_LIFEDRAIN},
	{name = "combat", interval = 2000, chance = 20, minDamage = -150, maxDamage = -250, radius = 1, shootEffect = CONST_ANI_SUDDENDEATH, effect = CONST_ME_PURPLEENERGY, target = true, type = COMBAT_LIFEDRAIN},
}

monster.defenses = {
	defense = 36,
	armor = 34,
}

monster.elements = {
	{type = COMBAT_FIREDAMAGE, percent = 70},
	{type = COMBAT_PHYSICALDAMAGE, percent = -25},
}

monster.immunities = {
	{type = "earth", combat = true, condition = true},
	{type = "paralyze", combat = false, condition = true},
}


mType:register(monster)