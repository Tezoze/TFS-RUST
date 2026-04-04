local mType = Game.createMonsterType("Cockroach")
local monster = {}

monster.description = "a cockroach"
monster.experience = 0
monster.outfit = {
	lookType = 284,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 8593
monster.health = 1
monster.maxHealth = 1
monster.race = "venom"
monster.speed = 180
monster.manaCost = 0
monster.maxSummons = 0

monster.changeTarget = {
	interval = 60000,
	chance = 0
}

monster.flags = {
	summonable = false,
	attackable = true,
	hostile = false,
	illusionable = false,
	convinceable = false,
	pushable = true,
	canPushItems = false,
	staticAttackChance = 50,
	targetDistance = 5,
	runHealth = 1,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = false,
}

monster.loot = {
	{id = 8710, chance = 100000}, -- cockroach leg
}


mType:register(monster)