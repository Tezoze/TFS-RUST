local mType = Game.createMonsterType("Rotworm Queen")
local monster = {}

monster.description = "a rotworm queen"
monster.experience = 75
monster.outfit = {
	lookType = 295,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 8947
monster.health = 105
monster.maxHealth = 105
monster.race = "blood"
monster.speed = 126
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
	canWalkOnEnergy = true,
	canWalkOnFire = true,
	canWalkOnPoison = true
}

monster.loot = {
	{id = 2148, chance = 100000, maxCount = 47}, -- gold coin
	{id = 8971, chance = 3333}, -- gland
	{id = 3976, chance = 20000, maxCount = 45}, -- worm
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = -5, maxDamage = -80, target = false},
}

monster.defenses = {
	defense = 15,
	armor = 10,
}


mType:register(monster)