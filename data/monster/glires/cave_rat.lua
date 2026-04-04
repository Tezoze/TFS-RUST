local mType = Game.createMonsterType("Cave Rat")
local monster = {}

monster.description = "a cave rat"
monster.experience = 10
monster.outfit = {
	lookType = 56,
	lookHead = 0,
	lookBody = 0,
	lookLegs = 0,
	lookFeet = 0,
	lookAddons = 0,
	lookMount = 0
}

monster.corpse = 5964
monster.health = 30
monster.maxHealth = 30
monster.race = "blood"
monster.speed = 150
monster.manaCost = 250
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
	pushable = true,
	canPushItems = false,
	canPushCreatures = false,
	targetDistance = 1,
	staticAttackChance = 90,
	runHealth = 3,
	canWalkOnEnergy = false,
	canWalkOnFire = false,
	canWalkOnPoison = false,
}

monster.voices = {
	interval = 5000,
	chance = 10,
	{text = "Meeeeep!", yell = false},
	{text = "Meep!", yell = false},
}

monster.loot = {
	{id = 2148, chance = 85000, maxCount = 2}, -- gold coin
	{id = 2687, chance = 750}, -- cookie
	{id = 2696, chance = 30000}, -- cheese
	{id = 3976, chance = 9700, maxCount = 2}, -- worm
}

monster.attacks = {
	{name = "melee", interval = 2000, minDamage = 0, maxDamage = -10, target = false},
}

monster.defenses = {
	defense = 5,
	armor = 1,
}

monster.elements = {
	{type = COMBAT_FIREDAMAGE, percent = -10},
}


mType:register(monster)