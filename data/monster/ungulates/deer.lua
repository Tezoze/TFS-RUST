local mType = Game.createMonsterType("Deer")
local monster = {}

monster.description = "a deer"
monster.experience = 0
monster.outfit = {
	lookType = 31,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 5970
monster.health = 25
monster.maxHealth = 25
monster.race = "blood"
monster.speed = 196
monster.manaCost = 260
monster.maxSummons = 0

monster.changeTarget = {
	interval = 4000,
	chance = 20
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
	targetDistance = 7,
	staticAttackChance = 0,
	runHealth = 25,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = false,
}

monster.loot = {
	{id = 2666, chance = 80000, maxCount = 4}, -- meat
	{id = 2671, chance = 50000, maxCount = 2}, -- ham
	{id = 11214, chance = 870}, -- antlers
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -1, target = false},
}

monster.defenses = {
	defense = 5,
	armor = 2,
}


mType:register(monster)