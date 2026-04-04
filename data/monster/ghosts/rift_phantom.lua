local mType = Game.createMonsterType("Rift Phantom")
local monster = {}

monster.description = "a rift phantom"
monster.experience = 0
monster.outfit = {
	lookType = 48,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 5993
monster.health = 150
monster.maxHealth = 150
monster.race = "undead"
monster.speed = 160
monster.manaCost = 100
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
	targetDistance = 1,
	staticAttackChance = 90,
	runHealth = 0,
	canWalkOnEnergy = true,
	canWalkOnFire = true,
	canWalkOnPoison = true
}


mType:register(monster)