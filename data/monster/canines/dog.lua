local mType = Game.createMonsterType("Dog")
local monster = {}

monster.description = "a dog"
monster.experience = 0
monster.outfit = {
	lookType = 32,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 5971
monster.health = 20
monster.maxHealth = 20
monster.race = "blood"
monster.speed = 124
monster.manaCost = 220
monster.maxSummons = 0

monster.changeTarget = {
	interval = 4000,
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
	targetDistance = 1,
	staticAttackChance = 90,
	runHealth = 8,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = false,
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Wuff wuff", yell = false},
}

monster.defenses = {
	defense = 5,
	armor = 5,
}


mType:register(monster)