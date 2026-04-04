local mType = Game.createMonsterType("Pink Butterfly")
local monster = {}

monster.description = "a butterfly"
monster.experience = 0
monster.outfit = {
	lookType = 213,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
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
	interval = 5000,
	chance = 20
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
	targetDistance = 6,
	runHealth = 2,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = false,
}


mType:register(monster)