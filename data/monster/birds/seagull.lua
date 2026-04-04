local mType = Game.createMonsterType("Seagull")
local monster = {}

monster.description = "a seagull"
monster.experience = 0
monster.outfit = {
	lookType = 223,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6076
monster.health = 25
monster.maxHealth = 25
monster.race = "blood"
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
	illusionable = false,
	convinceable = false,
	pushable = true,
	canPushItems = false,
	canPushCreatures = false,
	staticAttackChance = 90,
	targetDistance = 11,
	runHealth = 25,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = false,
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -3, target = false},
}

monster.defenses = {
	defense = 5,
	armor = 2,
}


mType:register(monster)