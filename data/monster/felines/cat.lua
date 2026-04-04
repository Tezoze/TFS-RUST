local mType = Game.createMonsterType("Cat")
local monster = {}

monster.description = "a cat"
monster.experience = 0
monster.outfit = {
	lookType = 276,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 7637
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
	runHealth = 8,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = false,
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Mew!", yell = false},
	{text = "Meow!", yell = false},
	{text = "Meow meow!", yell = false},
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = 0, target = false},
}

monster.defenses = {
	defense = 5,
	armor = 1,
}


mType:register(monster)