local mType = Game.createMonsterType("Silver Rabbit")
local monster = {}

monster.description = "a silver rabbit"
monster.experience = 0
monster.outfit = {
	lookType = 262,
	lookHead = 69,
	lookBody = 66,
	lookLegs = 69,
	lookFeet = 66,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 7338
monster.health = 15
monster.maxHealth = 15
monster.race = "blood"
monster.speed = 184
monster.manaCost = 220
monster.maxSummons = 0

monster.changeTarget = {
	interval = 4000,
	chance = 0
}

monster.flags = {
	summonable = true,
	attackable = true,
	hostile = true,
	illusionable = false,
	convinceable = true,
	pushable = false,
	canPushItems = false,
	staticAttackChance = 70,
	targetDistance = 1,
	runHealth = 15,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = false,
}

monster.loot = {
	{id = 2666, chance = 86170, maxCount = 2}, -- meat
	{id = 2684, chance = 11150}, -- carrot
	{id = 11209, chance = 28670}, -- silky fur
}

monster.defenses = {
	defense = 5,
	armor = 1,
}


mType:register(monster)