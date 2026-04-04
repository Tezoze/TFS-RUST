local mType = Game.createMonsterType("Rabbit")
local monster = {}

monster.description = "a rabbit"
monster.experience = 0
monster.outfit = {
	lookType = 74,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6017
monster.health = 15
monster.maxHealth = 15
monster.race = "blood"
monster.speed = 180
monster.manaCost = 220
monster.maxSummons = 0

monster.changeTarget = {
	interval = 5000,
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
	targetDistance = 11,
	staticAttackChance = 0,
	runHealth = 15,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = false,
}

monster.loot = {
	{id = 2666, chance = 85620, maxCount = 2}, -- meat
	{id = 2684, chance = 10000, maxCount = 2}, -- carrot
}

monster.defenses = {
	defense = 5,
	armor = 1,
}


mType:register(monster)