local mType = Game.createMonsterType("Northern Pike")
local monster = {}

monster.description = "a northern pike"
monster.experience = 0
monster.outfit = {
	lookType = 454,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 2669
monster.health = 95
monster.maxHealth = 95
monster.race = "undead"
monster.speed = 180
monster.manaCost = 0
monster.maxSummons = 0

monster.changeTarget = {
	interval = 4000,
	chance = 0
}

monster.flags = {
	summonable = false,
	attackable = true,
	hostile = true,
	illusionable = false,
	convinceable = false,
	pushable = true,
	canPushItems = true,
	canPushCreatures = true,
	targetDistance = 1,
	staticAttackChance = 90,
	runHealth = 95,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = true
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Blub!", yell = false},
}

monster.defenses = {
	defense = 5,
	armor = 5,
}

monster.immunities = {
	{type = "earth", combat = true, condition = true},
	{type = "fire", combat = true, condition = true},
	{type = "invisible", combat = false, condition = true},
	{type = "drown", combat = true, condition = true},
}


mType:register(monster)
