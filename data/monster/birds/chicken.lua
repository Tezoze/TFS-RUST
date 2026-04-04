local mType = Game.createMonsterType("Chicken")
local monster = {}

monster.description = ""
monster.experience = 0
monster.outfit = {
	lookType = 111,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6042
monster.health = 15
monster.maxHealth = 15
monster.race = "blood"
monster.speed = 128
monster.manaCost = 220
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
	targetDistance = 7,
	staticAttackChance = 0,
	runHealth = 15,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = false,
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Gokgoooook", yell = false},
	{text = "Cluck Cluck", yell = false},
}

monster.loot = {
	{id = 2666, chance = 2120, maxCount = 2}, -- meat
	{id = 2695, chance = 950}, -- egg
	{id = 3976, chance = 10000, maxCount = 3}, -- worm
	{id = 5890, chance = 20000, maxCount = 3}, -- chicken feather
}

monster.defenses = {
	defense = 5,
	armor = 5,
}


mType:register(monster)