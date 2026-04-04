local mType = Game.createMonsterType("The Hairy One")
local monster = {}

monster.description = "The Hairy One"
monster.experience = 115
monster.outfit = {
	lookType = 116,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 7349
monster.health = 325
monster.maxHealth = 325
monster.race = "blood"
monster.speed = 240
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
	{text = "Hugah!", yell = false},
	{text = "Ungh! Ungh!", yell = false},
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -70, target = false},
}

monster.defenses = {
	defense = 0,
	armor = 16,
}

monster.elements = {
	{type = COMBAT_ENERGYDAMAGE, percent = 5},
	{type = COMBAT_FIREDAMAGE, percent = -20},
}

monster.immunities = {
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)