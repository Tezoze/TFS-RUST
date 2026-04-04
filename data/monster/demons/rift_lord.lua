local mType = Game.createMonsterType("Rift Lord")
local monster = {}

monster.description = "a rift lord"
monster.experience = 0
monster.outfit = {
	lookType = 12,
	lookHead = 9,
	lookBody = 19,
	lookLegs = 9,
	lookFeet = 85,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 0
monster.health = 5
monster.maxHealth = 5
monster.race = "fire"
monster.speed = 200
monster.manaCost = 0
monster.maxSummons = 0

monster.flags = {
	summonable = false,
	attackable = false,
	hostile = false,
	illusionable = false,
	convinceable = false,
	pushable = false,
	canPushItems = true,
	canPushCreatures = false,
	staticAttackChance = 98,
	targetDistance = 1,
	runHealth = 0,
	canWalkOnEnergy = true,
	canWalkOnFire = true,
	canWalkOnPoison = true
}


mType:register(monster)