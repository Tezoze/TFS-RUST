local mType = Game.createMonsterType("Pig")
local monster = {}

monster.description = "a pig"
monster.experience = 0
monster.outfit = {
	lookType = 60,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6000
monster.health = 25
monster.maxHealth = 25
monster.race = "blood"
monster.speed = 114
monster.manaCost = 255
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
	runHealth = 25,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = false,
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Oink oink", yell = false},
	{text = "Oink", yell = false},
}

monster.loot = {
	{id = 2666, chance = 64000, maxCount = 4}, -- meat
	{id = 10610, chance = 950}, -- pig foot
}


mType:register(monster)