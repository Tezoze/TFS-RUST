local mType = Game.createMonsterType("butterfly")
local monster = {}

monster.description = ""
monster.experience = 0
monster.outfit = {
	lookType = 227,
	lookHead = 20,
	lookBody = 30,
	lookLegs = 40,
	lookFeet = 50,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 4313
monster.health = 2
monster.maxHealth = 2
monster.race = "venom"
monster.speed = 320
monster.manaCost = 0
monster.maxSummons = 0

monster.changeTarget = {
	interval = 4000,
	chance = 8
}

monster.flags = {
	summonable = false,
	attackable = true,
	hostile = false,
	illusionable = false,
	convinceable = false,
	pushable = false,
	canPushItems = true,
	canPushCreatures = false,
	staticAttackChance = 0,
	targetDistance = 8,
	runHealth = 2,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = false,
}


mType:register(monster)