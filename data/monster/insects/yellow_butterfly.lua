local mType = Game.createMonsterType("Yellow Butterfly")
local monster = {}

monster.description = "a butterfly"
monster.experience = 0
monster.outfit = {
	lookType = 10,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 5014
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
	illusionable = true,
	convinceable = false,
	pushable = true,
	canPushItems = false,
	canPushCreatures = false,
	targetDistance = 6,
	staticAttackChance = 0,
	runHealth = 2,
	canWalkOnEnergy = true,
	canWalkOnFire = true,
	canWalkOnPoison = true
}


mType:register(monster)