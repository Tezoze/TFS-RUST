local mType = Game.createMonsterType("Penguin")
local monster = {}

monster.description = "a penguin"
monster.experience = 1
monster.outfit = {
	lookType = 250,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 7334
monster.health = 33
monster.maxHealth = 33
monster.race = "blood"
monster.speed = 116
monster.manaCost = 300
monster.maxSummons = 0

monster.changeTarget = {
	interval = 4000,
	chance = 0
}

monster.flags = {
	summonable = true,
	attackable = true,
	hostile = true,
	illusionable = true,
	convinceable = true,
	pushable = false,
	canPushItems = false,
	canPushCreatures = false,
	targetDistance = 1,
	runHealth = 32,
	staticAttackChance = 90,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = false,
}

monster.loot = {
	{id = 2667, chance = 7500, maxCount = 2}, -- fish
	{id = 7158, chance = 60}, -- rainbow trout
	{id = 7159, chance = 140}, -- green perch
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -3, target = false},
}

monster.defenses = {
	defense = 5,
	armor = 2,
}

monster.elements = {
	{type = COMBAT_ICEDAMAGE, percent = 5},
	{type = COMBAT_ENERGYDAMAGE, percent = -5},
}


mType:register(monster)