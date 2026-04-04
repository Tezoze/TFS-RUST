local mType = Game.createMonsterType("Mimic")
local monster = {}

monster.description = "a mimic"
monster.experience = 0
monster.outfit = {
	lookType = 92,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 1740
monster.health = 30
monster.maxHealth = 30
monster.race = "blood"
monster.speed = 170
monster.manaCost = 0
monster.maxSummons = 0

monster.changeTarget = {
	interval = 5000,
	chance = 8
}

monster.flags = {
	summonable = false,
	attackable = true,
	hostile = true,
	illusionable = false,
	convinceable = false,
	pushable = true,
	canPushItems = false,
	canPushCreatures = false,
	targetDistance = 5,
	staticAttackChance = 0,
	runHealth = 0,
	canWalkOnEnergy = true,
	canWalkOnFire = true,
	canWalkOnPoison = true
}

monster.defenses = {
	defense = 3,
	armor = 2,
}


mType:register(monster)