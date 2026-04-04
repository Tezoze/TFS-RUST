local mType = Game.createMonsterType("Frostfur")
local monster = {}

monster.description = "Frostfur"
monster.experience = 35
monster.outfit = {
	lookType = 52,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 7349
monster.health = 65
monster.maxHealth = 65
monster.race = "blood"
monster.speed = 170
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

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -30, target = false},
}

monster.defenses = {
	defense = 3,
	armor = 5,
}

monster.elements = {
	{type = COMBAT_FIREDAMAGE, percent = 20},
	{type = COMBAT_ENERGYDAMAGE, percent = -10},
}

monster.immunities = {
	{type = "outfit", combat = false, condition = true},
	{type = "drunk", combat = false, condition = true},
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)