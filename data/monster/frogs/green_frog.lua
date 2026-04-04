local mType = Game.createMonsterType("Green Frog")
local monster = {}

monster.description = "a green frog"
monster.experience = 0
monster.outfit = {
	lookType = 224,
	lookHead = 69,
	lookBody = 66,
	lookLegs = 69,
	lookFeet = 66,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6079
monster.health = 25
monster.maxHealth = 25
monster.race = "venom"
monster.speed = 320
monster.manaCost = 250
monster.maxSummons = 0

monster.changeTarget = {
	interval = 5000,
	chance = 0
}

monster.flags = {
	summonable = true,
	attackable = true,
	hostile = true,
	illusionable = true,
	convinceable = true,
	pushable = true,
	canPushItems = false,
	canPushCreatures = false,
	staticAttackChance = 90,
	targetDistance = 6,
	runHealth = 0,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = false,
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Ribbit!", yell = false},
	{text = "Ribbit! Ribbit!", yell = false},
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -25, target = false},
}

monster.defenses = {
	defense = 5,
	armor = 2,
}


mType:register(monster)