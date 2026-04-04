local mType = Game.createMonsterType("Carrion Worm")
local monster = {}

monster.description = "a carrion worm"
monster.experience = 70
monster.outfit = {
	lookType = 192,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 6069
monster.health = 145
monster.maxHealth = 145
monster.race = "blood"
monster.speed = 130
monster.manaCost = 380
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
	convinceable = true,
	pushable = false,
	canPushItems = true,
	canPushCreatures = false,
	targetDistance = 1,
	staticAttackChance = 80,
	runHealth = 0,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = false,
}

monster.loot = {
	{id = 2148, chance = 50000, maxCount = 45}, -- gold coin
	{id = 2666, chance = 9460, maxCount = 2}, -- meat
	{id = 3976, chance = 2100, maxCount = 2}, -- worm
	{id = 11192, chance = 10000}, -- carrion worm fang
	{id = 8748, chance = 210},
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -45, target = false},
}

monster.defenses = {
	defense = 15,
	armor = 8,
}

monster.elements = {
	{type = COMBAT_EARTHDAMAGE, percent = 20},
	{type = COMBAT_ENERGYDAMAGE, percent = 10},
	{type = COMBAT_FIREDAMAGE, percent = -5},
	{type = COMBAT_ICEDAMAGE, percent = -5},
}


mType:register(monster)