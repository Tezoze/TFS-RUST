local mType = Game.createMonsterType("Rift Worm")
local monster = {}

monster.description = "a rift worm"
monster.experience = 1195
monster.outfit = {
	lookType = 295,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 0
monster.health = 2800
monster.maxHealth = 2800
monster.race = "blood"
monster.speed = 200
monster.manaCost = 0
monster.maxSummons = 0

monster.changeTarget = {
	interval = 60000,
	chance = 0
}

monster.flags = {
	summonable = false,
	attackable = true,
	hostile = true,
	illusionable = false,
	convinceable = false,
	pushable = false,
	canPushItems = false,
	staticAttackChance = 50,
	targetDistance = 1,
	runHealth = 0,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = false,
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -160, target = false},
	{name = "combat", interval = 2000, chance = 10, minDamage = -500, maxDamage = -1000, range = 7, target = false, type = COMBAT_MANADRAIN},
}

monster.defenses = {
	defense = 25,
	armor = 25,
	{name = "invisible", interval = 4000, chance = 15, effect = CONST_ME_GROUNDSHAKER},
}

monster.elements = {
	{type = COMBAT_HOLYDAMAGE, percent = -5},
	{type = COMBAT_FIREDAMAGE, percent = -5},
	{type = COMBAT_ICEDAMAGE, percent = -5},
	{type = COMBAT_EARTHDAMAGE, percent = -25},
}

monster.immunities = {
	{type = "invisible", combat = false, condition = true},
}


mType:register(monster)