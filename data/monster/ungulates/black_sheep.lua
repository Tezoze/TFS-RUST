local mType = Game.createMonsterType("Black Sheep")
local monster = {}

monster.description = ""
monster.experience = 0
monster.outfit = {
	lookType = 13,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 5994
monster.health = 20
monster.maxHealth = 20
monster.race = "blood"
monster.speed = 116
monster.manaCost = 250
monster.maxSummons = 0

monster.changeTarget = {
	interval = 4000,
	chance = 20
}

monster.flags = {
	summonable = true,
	attackable = true,
	hostile = false,
	illusionable = true,
	convinceable = true,
	pushable = true,
	canPushItems = false,
	canPushCreatures = false,
	targetDistance = 7,
	staticAttackChance = 0,
	runHealth = 20,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = false,
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Maeh", yell = false},
}

monster.loot = {
	{id = 2666, chance = 70860, maxCount = 5}, -- meat
	{id = 12404, chance = 1000}, -- black wool
}

monster.defenses = {
	defense = 5,
	armor = 5,
}


mType:register(monster)