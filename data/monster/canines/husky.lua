local mType = Game.createMonsterType("Husky")
local monster = {}

monster.description = "a husky"
monster.experience = 0
monster.outfit = {
	lookType = 258,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 7316
monster.health = 140
monster.maxHealth = 140
monster.race = "blood"
monster.speed = 264
monster.manaCost = 420
monster.maxSummons = 0

monster.changeTarget = {
	interval = 4000,
	chance = 0
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
	targetDistance = 1,
	staticAttackChance = 90,
	runHealth = 0,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = false,
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Yoooohuuuu!", yell = false},
	{text = "Grrrrrrr", yell = false},
	{text = "Ruff, ruff!", yell = false},
}

monster.defenses = {
	defense = 5,
	armor = 1,
}


mType:register(monster)